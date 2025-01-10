use avail_base::metrics::avail::{MetricObserver, ObserveKind};
use avail_core::{
	data_proof::ProofResponse, header::HeaderExtension, traits::ExtendedHeader, OpaqueExtrinsic,
};
use da_runtime::apis::{DataAvailApi, KateApi as RTKateApi};
use da_runtime::kate::{GDataProof, GRow};
use kate::com::Cell;

use frame_support::BoundedVec;
use frame_system::limits::BlockLength;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
	types::error::{ErrorCode, ErrorObject},
};
use sc_client_api::BlockBackend;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_runtime::{
	generic::SignedBlock,
	traits::{Block as BlockT, ConstU32, Header},
};
use std::{marker::PhantomData, marker::Sync, sync::Arc};

pub type HashOf<Block> = <Block as BlockT>::Hash;
pub type MaxRows = ConstU32<64>;
pub type Rows = BoundedVec<u32, MaxRows>;
pub type MaxCells = ConstU32<10_000>;
pub type Cells = BoundedVec<Cell, MaxCells>;

#[cfg(feature = "grandpa-justifications")]
pub mod justifications;
pub mod metrics;

/// # TODO
/// - [ ] Update type definitions for RPCs in our subxt & explorer.
#[rpc(client, server)]
pub trait KateApi<Block>
where
	Block: BlockT,
{
	#[method(name = "kate_queryRows")]
	async fn query_rows(&self, rows: Rows, at: Option<HashOf<Block>>) -> RpcResult<Vec<GRow>>;

	#[method(name = "kate_queryProof")]
	async fn query_proof(
		&self,
		cells: Cells,
		at: Option<HashOf<Block>>,
	) -> RpcResult<Vec<GDataProof>>;

	#[method(name = "kate_blockLength")]
	async fn query_block_length(&self, at: Option<HashOf<Block>>) -> RpcResult<BlockLength>;

	#[method(name = "kate_queryDataProof")]
	async fn query_data_proof(
		&self,
		transaction_index: u32,
		at: Option<HashOf<Block>>,
	) -> RpcResult<ProofResponse>;
}

#[allow(clippy::type_complexity)]
pub struct Kate<Client, Block: BlockT> {
	client: Arc<Client>,
	max_cells_size: usize,
	_block: PhantomData<Block>,
}

impl<Client, Block: BlockT> Kate<Client, Block> {
	pub fn new(client: Arc<Client>, max_cells_size: usize) -> Self {
		Self {
			client,
			max_cells_size,
			_block: PhantomData,
		}
	}
}

/// Error type of this RPC api.
pub enum Error {
	/// The transaction was not decodable.
	KateRPCError,
}

impl From<Error> for i32 {
	fn from(e: Error) -> i32 {
		match e {
			Error::KateRPCError => 1,
		}
	}
}

macro_rules! internal_err {
	($($arg:tt)*) => {{
		ErrorObject::owned(
			Error::KateRPCError.into(),
			format!($($arg)*),
			None::<()>
		)
	}}
}

// ApiRef<'_, dyn ApiExt<Block>>,

type Opaques<B> = Vec<<B as BlockT>::Extrinsic>;
type Api<'a, C, B> = ApiRef<'a, <C as ProvideRuntimeApi<B>>::Api>;

impl<Client, Block> Kate<Client, Block>
where
	Block: BlockT,
	<Block as BlockT>::Header: ExtendedHeader<Extension = HeaderExtension>,
	Client: Send + Sync + 'static,
	Client: HeaderBackend<Block> + ProvideRuntimeApi<Block> + BlockBackend<Block>,
	Client::Api: DataAvailApi<Block>,
	// Extrinsic: TryFrom<<Block as BlockT>::Extrinsic>,
{
	#[allow(clippy::type_complexity)]
	fn scope(
		&self,
		at: Option<Block::Hash>,
	) -> RpcResult<(
		Api<'_, Client, Block>,
		<Block as BlockT>::Hash,
		u32,
		BlockLength,
		Opaques<Block>,
		<Block as BlockT>::Header,
	)> {
		let at = self.at_or_best(at);
		let block = self.get_finalized_block(Some(at))?.block;
		let number: u32 = (*block.header().number())
			.try_into()
			.map_err(|_| ErrorCode::InvalidParams)?;
		let (header, extrinsics) = block.deconstruct();

		let api = self.client.runtime_api();
		let block_len = api
			.block_length(at)
			.map_err(|e| internal_err!("Length of best block({at:?}): {e:?}"))?;

		Ok((api, at, number, block_len, extrinsics, header))
	}

	fn at_or_best(&self, at: Option<<Block as BlockT>::Hash>) -> <Block as BlockT>::Hash {
		at.unwrap_or_else(|| self.client.info().best_hash)
	}

	fn ensure_block_finalized(&self, block: &SignedBlock<Block>) -> RpcResult<()> {
		let block_header = block.block.header();
		let (block_hash, block_number) = (block_header.hash(), *block_header.number());

		if self.client.info().finalized_number < block_number {
			return Err(internal_err!(
				"Requested block {block_hash} is not finalized"
			));
		}

		Ok(())
	}

	fn get_block(&self, at: Option<Block::Hash>) -> RpcResult<SignedBlock<Block>> {
		let at = self.at_or_best(at);
		self.client
			.block(at)
			.map_err(|e| internal_err!("Invalid block number: {:?}", e))?
			.ok_or_else(|| internal_err!("Missing block {}", at))
	}

	fn get_finalized_block(&self, at: Option<Block::Hash>) -> RpcResult<SignedBlock<Block>> {
		let signed_block = self.get_block(at)?;
		self.ensure_block_finalized(&signed_block)?;
		Ok(signed_block)
	}
}

#[async_trait]
impl<Client, Block> KateApiServer<Block> for Kate<Client, Block>
where
	Block: BlockT<Extrinsic = OpaqueExtrinsic>,
	<Block as BlockT>::Header: ExtendedHeader<Extension = HeaderExtension>,
	Client: Send + Sync + 'static,
	Client: HeaderBackend<Block> + ProvideRuntimeApi<Block> + BlockBackend<Block>,
	Client::Api: DataAvailApi<Block> + RTKateApi<Block>,
{
	async fn query_rows(&self, rows: Rows, at: Option<HashOf<Block>>) -> RpcResult<Vec<GRow>> {
		let _metric_observer = MetricObserver::new(ObserveKind::KateQueryRows);

		let (api, at, number, block_len, extrinsics, header) = self.scope(at)?;

		match header.extension() {
			HeaderExtension::V3(ext) => {
				if ext.commitment.commitment.is_empty() {
					return Err(internal_err!("Requested block {at} has empty commitments"));
				}
			},
		};

		let grid_rows = api
			.rows(at, number, extrinsics, block_len, rows.into())
			.map_err(|kate_err| internal_err!("Failed Kate rows: {kate_err:?}"))?
			.map_err(|api_err| internal_err!("Failed API: {api_err:?}"))?;

		Ok(grid_rows)
	}

	async fn query_proof(
		&self,
		cells: Cells,
		at: Option<HashOf<Block>>,
	) -> RpcResult<Vec<GDataProof>> {
		if cells.len() > self.max_cells_size {
			return Err(
				internal_err!(
					"Cannot query ({}) more than {} amount of cells per request. Either increase the max cells size (--kate-max-cells-size) or query less amount of cells per request.",
					cells.len(),
					self.max_cells_size
				)
			);
		}

		let _metric_observer = MetricObserver::new(ObserveKind::KateQueryProof);

		let (api, at, number, block_len, extrinsics, header) = self.scope(at)?;
		match header.extension() {
			HeaderExtension::V3(ext) => {
				if ext.commitment.commitment.is_empty() {
					return Err(internal_err!("Requested block {at} has empty commitments"));
				}
			},
		};

		let cells = cells
			.into_iter()
			.map(|cell| (cell.row.0, cell.col.0))
			.collect::<Vec<_>>();
		let proof = api
			.proof(at, number, extrinsics, block_len, cells)
			.map_err(|kate_err| internal_err!("KateApi::proof failed: {kate_err:?}"))?
			.map_err(|api_err| internal_err!("Failed API: {api_err:?}"))?;

		Ok(proof)
	}

	async fn query_block_length(&self, at: Option<HashOf<Block>>) -> RpcResult<BlockLength> {
		let _metric_observer = MetricObserver::new(ObserveKind::KateQueryBlockLength);

		let at = self.at_or_best(at);
		let api = self.client.runtime_api();
		let block_length = api
			.block_length(at)
			.map_err(|e| internal_err!("Length of best block({at:?}): {e:?}"))?;

		Ok(block_length)
	}

	async fn query_data_proof(
		&self,
		tx_idx: u32,
		at: Option<HashOf<Block>>,
	) -> RpcResult<ProofResponse> {
		let _metric_observer = MetricObserver::new(ObserveKind::KateQueryDataProof);

		// Calculate proof for block and tx index
		let (api, at, number, _, extrinsics, _) = self.scope(at)?;
		let proof = api
			.data_proof(at, number, extrinsics, tx_idx)
			.map_err(|e| internal_err!("KateApi::data_proof failed: {e:?}"))?
			.ok_or_else(|| {
				internal_err!("Cannot fetch tx data at tx index {tx_idx:?} at block {at:?}")
			})?;

		Ok(proof)
	}
}
