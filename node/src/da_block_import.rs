/// # Data Avail Protocol
///
/// This `BlockImport` ensures that any block follows the Data Availability Protocol before send it
/// to Babe and Grandpa.
/// It double-checks the **extension header** which contains the `Kate Commitment` and `Data
/// Root`.
use avail_base::metrics::avail::{MetricObserver, ObserveKind};
use avail_core::{
	block::header::HeaderExtension, ensure, BlockLengthColumns, BlockLengthRows, OpaqueExtrinsic,
	BLOCK_CHUNK_SIZE,
};
use da_runtime::{
	apis::{DataAvailApi, ExtensionBuilder},
	Header as DaHeader,
};
use frame_system::limits::BlockLength;

use sc_consensus::{
	block_import::{BlockCheckParams, BlockImport as BlockImportT, BlockImportParams},
	ImportResult,
};
use sp_api::{ApiError, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Error as ConsensusError};
use sp_core::H256;
use sp_runtime::traits::Block as BlockT;
use std::{marker::PhantomData, sync::Arc};

pub struct BlockImport<B, C, I> {
	client: Arc<C>,
	inner: I,
	// If true, it skips the DA block import check during sync only.
	unsafe_da_sync: bool,
	_block: PhantomData<B>,
}

impl<B, C, I> BlockImport<B, C, I>
where
	B: BlockT<Extrinsic = OpaqueExtrinsic, Header = DaHeader, Hash = H256>,
	I: BlockImportT<B> + Clone + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + Send + Sync,
	C::Api: DataAvailApi<B> + ExtensionBuilder<B>,
{
	pub fn new(client: Arc<C>, inner: I, unsafe_da_sync: bool) -> Self {
		Self {
			client,
			inner,
			unsafe_da_sync,
			_block: PhantomData,
		}
	}

	fn ensure_last_extrinsic_is_failed_send_message_txs(
		&self,
		block: &BlockImportParams<B>,
	) -> Result<(), ConsensusError> {
		let err = block_doesnt_contain_post_inherent();

		let maybe_body = block.body.as_ref();
		let Some(body) = maybe_body else {
			return Err(err);
		};

		let Some(last_extrinsic) = body.last() else {
			return Err(err);
		};

		let parent_hash = <B as BlockT>::Hash::from(block.header.parent_hash);
		let api = self.client.runtime_api();

		let Ok(found) = api.check_if_extrinsic_is_post_inherent(parent_hash, last_extrinsic) else {
			return Err(err);
		};

		ensure!(found, err);

		Ok(())
	}

	fn ensure_valid_header_extension(
		&self,
		block: &BlockImportParams<B>,
	) -> Result<(), ConsensusError> {
		let block_len = extension_block_len(&block.header.extension);
		let extrinsics = || block.body.clone().unwrap_or_default();
		let block_number: u32 = block.header.number;
		let parent_hash = <B as BlockT>::Hash::from(block.header.parent_hash);
		let api = self.client.runtime_api();

		// Calculate data root and extension.
		let data_root = api
			.build_data_root(parent_hash, block_number, extrinsics())
			.map_err(data_root_fail)?;
		let extension = api
			.build_extension(
				parent_hash,
				extrinsics(),
				data_root,
				block_len,
				block_number,
			)
			.map_err(build_ext_fail)?;

		// Check equality between calculated and imported extensions.
		ensure!(
			block.header.extension == extension,
			extension_mismatch(&block.header.extension, &extension)
		);
		Ok(())
	}
}

#[async_trait::async_trait]
impl<B, C, I> BlockImportT<B> for BlockImport<B, C, I>
where
	B: BlockT<Extrinsic = OpaqueExtrinsic, Header = DaHeader, Hash = H256>,
	I: BlockImportT<B> + Clone + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + Send + Sync,
	C::Api: DataAvailApi<B> + ExtensionBuilder<B>,
{
	type Error = ConsensusError;

	/// It verifies that header extension (Kate commitment & data root) is properly calculated.
	async fn import_block(
		&mut self,
		block: BlockImportParams<B>,
	) -> Result<ImportResult, Self::Error> {
		let _metric_observer = MetricObserver::new(ObserveKind::ImportBlockTotalExecutionTime);

		// We only want to check for blocks that are not from "Own"
		let is_own = matches!(block.origin, BlockOrigin::Own);

		// We skip checks if we're syncing and unsafe_da_sync is true
		let is_sync = matches!(
			block.origin,
			BlockOrigin::NetworkInitialSync | BlockOrigin::File
		);
		let skip_sync = self.unsafe_da_sync && is_sync;
		if !is_own && !skip_sync && !block.with_state() {
			self.ensure_last_extrinsic_is_failed_send_message_txs(&block)?;
			self.ensure_valid_header_extension(&block)?;
		}

		// Next import block stage & metrics
		let result = self.inner.import_block(block).await;
		result.map_err(Into::into)
	}

	async fn check_block(
		&mut self,
		block: BlockCheckParams<B>,
	) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(block).await.map_err(Into::into)
	}
}

impl<B, C, I: Clone> Clone for BlockImport<B, C, I> {
	fn clone(&self) -> Self {
		Self {
			client: self.client.clone(),
			inner: self.inner.clone(),
			unsafe_da_sync: self.unsafe_da_sync,
			_block: PhantomData,
		}
	}
}

/// Calculate block length from `extension`.
fn extension_block_len(extension: &HeaderExtension) -> BlockLength {
	BlockLength::with_normal_ratio(
		BlockLengthRows(extension.rows() as u32),
		BlockLengthColumns(extension.cols() as u32),
		BLOCK_CHUNK_SIZE,
		sp_runtime::Perbill::from_percent(90),
	)
	.expect("Valid BlockLength at genesis .qed")
}

fn extension_mismatch(imported: &HeaderExtension, generated: &HeaderExtension) -> ConsensusError {
	let msg =
		format!("DA Extension does NOT match\nExpected: {imported:#?}\nGenerated:{generated:#?}");
	ConsensusError::ClientImport(msg)
}

fn data_root_fail(e: ApiError) -> ConsensusError {
	let msg = format!("Data root cannot be calculated: {e:?}");
	ConsensusError::ClientImport(msg)
}

fn build_ext_fail(e: ApiError) -> ConsensusError {
	let msg = format!("Build extension fails due to: {e:?}");
	ConsensusError::ClientImport(msg)
}

fn block_doesnt_contain_post_inherent() -> ConsensusError {
	let msg = "Block does not contain post inherent".to_string();
	ConsensusError::ClientImport(msg)
}
