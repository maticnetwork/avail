use crate::{Cells, HashOf, Kate, KateApiServer, ProofResponse, Rows};

use avail_core::{block::header::HeaderExtension, block::ExtendedHeader, OpaqueExtrinsic};
use da_runtime::apis::DataAvailApi;

use crate::RTKateApi;
use da_runtime::kate::{GDataProof, GRow};
use frame_system::limits::BlockLength;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
};
use sc_client_api::BlockBackend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

#[rpc(client, server)]
pub trait KateApiMetrics<Block>
where
	Block: BlockT,
{
	#[method(name = "kate_queryRowsMetrics")]
	async fn query_rows_metrics(
		&self,
		rows: Rows,
		at: Option<HashOf<Block>>,
	) -> RpcResult<(Vec<GRow>, u128)>;

	#[method(name = "kate_queryProofMetrics")]
	async fn query_proof_metrics(
		&self,
		cells: Cells,
		at: Option<HashOf<Block>>,
	) -> RpcResult<(Vec<GDataProof>, u128)>;

	#[method(name = "kate_blockLengthMetrics")]
	async fn query_block_length_metrics(
		&self,
		at: Option<HashOf<Block>>,
	) -> RpcResult<(BlockLength, u128)>;

	#[method(name = "kate_queryDataProofMetrics")]
	async fn query_data_proof_metrics(
		&self,
		transaction_index: u32,
		at: Option<HashOf<Block>>,
	) -> RpcResult<(ProofResponse, u128)>;
}

#[async_trait]
impl<Client, Block> KateApiMetricsServer<Block> for Kate<Client, Block>
where
	Block: BlockT<Extrinsic = OpaqueExtrinsic>,
	<Block as BlockT>::Header: ExtendedHeader<Extension = HeaderExtension>,
	Client: Send + Sync + 'static,
	Client: HeaderBackend<Block> + ProvideRuntimeApi<Block> + BlockBackend<Block>,
	Client::Api: DataAvailApi<Block> + RTKateApi<Block>,
{
	async fn query_rows_metrics(
		&self,
		rows: Rows,
		at: Option<HashOf<Block>>,
	) -> RpcResult<(Vec<GRow>, u128)> {
		let start = std::time::Instant::now();
		let result = self.query_rows(rows, at).await;
		let elapsed = start.elapsed();

		result.map(|r| (r, elapsed.as_micros()))
	}

	async fn query_proof_metrics(
		&self,
		cells: Cells,
		at: Option<HashOf<Block>>,
	) -> RpcResult<(Vec<GDataProof>, u128)> {
		let start = std::time::Instant::now();
		let result = self.query_proof(cells, at).await;
		let elapsed = start.elapsed();

		result.map(|r| (r, elapsed.as_micros()))
	}

	async fn query_block_length_metrics(
		&self,
		at: Option<HashOf<Block>>,
	) -> RpcResult<(BlockLength, u128)> {
		let start = std::time::Instant::now();
		let result = self.query_block_length(at).await;
		let elapsed = start.elapsed();

		result.map(|r| (r, elapsed.as_micros()))
	}

	async fn query_data_proof_metrics(
		&self,
		transaction_index: u32,
		at: Option<HashOf<Block>>,
	) -> RpcResult<(ProofResponse, u128)> {
		let start = std::time::Instant::now();
		let result = self.query_data_proof(transaction_index, at).await;
		let elapsed = start.elapsed();

		result.map(|r| (r, elapsed.as_micros()))
	}
}
