use crate::primitives::kate::{Cells, Rows};
use crate::H256;
use avail_core::data_proof::ProofResponse;

use crate::avail::runtime_types::frame_system::limits::BlockLength;
use crate::from_substrate::{FeeDetails, NodeRole, PeerInfo, RuntimeDispatchInfo, SyncState};
use crate::{
	AvailBlockDetailsRPC, AvailConfig, AvailHeader, BlockHash, BlockNumber, Cell, GDataProof, GRow,
};
use subxt::backend::legacy::rpc_methods::{Bytes, LegacyRpcMethods, SystemHealth};
use subxt::backend::rpc::RpcClient;
use subxt::rpc_params;

pub use jsonrpsee::core::client::Client as JsonRpcCLient;
use jsonrpsee::proc_macros::rpc;

/// Arbitrary properties defined in chain spec as a JSON object
pub type Properties = serde_json::map::Map<String, serde_json::Value>;

mod jsonrpsee_helpers {
	pub use jsonrpsee::{
		client_transport::ws::{self, EitherStream, Url, WsTransportClientBuilder},
		core::client::{Client, Error},
	};
	use tokio_util::compat::Compat;

	pub type Sender = ws::Sender<Compat<EitherStream>>;
	pub type Receiver = ws::Receiver<Compat<EitherStream>>;

	/// Build WS RPC client from URL
	pub async fn client(url: &str) -> Result<Client, Error> {
		let (sender, receiver) = ws_transport(url).await?;
		Ok(Client::builder()
			.max_buffer_capacity_per_subscription(4096)
			.build_with_tokio(sender, receiver))
	}

	async fn ws_transport(url: &str) -> Result<(Sender, Receiver), Error> {
		let url = Url::parse(url).map_err(|e| Error::Transport(e.into()))?;
		WsTransportClientBuilder::default()
			.build(url)
			.await
			.map_err(|e| Error::Transport(e.into()))
	}
}

pub struct Rpc {
	pub client: RpcClient,
	pub methods: LegacyRpcMethods<AvailConfig>,
	pub rpc_methods: JsonRpcCLient,
	pub kate: Kate,
	pub author: Author,
	pub chain: Chain,
	pub system: System,
	pub payment: Payment,
}

impl Rpc {
	pub async fn new(endpoint: &str) -> Result<Self, Box<dyn std::error::Error>> {
		let client = RpcClient::from_insecure_url(endpoint).await?;
		let methods = LegacyRpcMethods::new(client.clone());
		let kate = Kate::new(client.clone());
		let author = Author::new(client.clone());
		let chain: Chain = Chain::new(client.clone());
		let system = System::new(client.clone());
		let payment = Payment::new(client.clone());

		let rpc_methods = jsonrpsee_helpers::client(endpoint.as_ref()).await.unwrap();
		Ok(Self {
			client,
			methods,
			rpc_methods,
			kate,
			author,
			chain,
			system,
			payment,
		})
	}
}

pub struct Payment {
	client: RpcClient,
}

impl Payment {
	pub fn new(client: RpcClient) -> Self {
		Self { client }
	}

	pub async fn query_fee_details(
		&self,
		extrinsic: Bytes,
		at: Option<BlockHash>,
	) -> Result<FeeDetails, subxt::Error> {
		let value: FeeDetails = self
			.client
			.request("payment_queryFeeDetails", rpc_params![extrinsic, at])
			.await?;
		Ok(value)
	}

	pub async fn query_info(
		&self,
		extrinsic: Bytes,
		at: Option<BlockHash>,
	) -> Result<RuntimeDispatchInfo, subxt::Error> {
		let value: RuntimeDispatchInfo = self
			.client
			.request("payment_queryInfo", rpc_params![extrinsic, at])
			.await?;
		Ok(value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::str::FromStr;

	#[tokio::test]
	async fn testing_function() {
		let sdk = crate::sdk::SDK::new("ws://127.0.0.1:9944").await.unwrap();
		let h = BlockHash::from_str(
			"0x90c6d99b3fb9d608a5bee9eb59bb107d5fd11b0aa398f3b1132503c15db40551",
		)
		.unwrap();
		let block = sdk.rpc.chain.get_block(Some(h)).await.unwrap();
		//dbg!(&block.block.extrinsics);
		let info = sdk
			.rpc
			.payment
			.query_info(block.block.extrinsics[1].clone(), Some(h))
			.await;

		match info {
			Ok(a) => {
				dbg!(&a);
			},
			Err(a) => {
				dbg!(a);
				panic!();
			},
		};
	}
}

pub struct System {
	client: RpcClient,
}

impl System {
	pub fn new(client: RpcClient) -> Self {
		Self { client }
	}

	pub async fn account_next_index(&self, account: String) -> Result<u32, subxt::Error> {
		let value: u32 = self
			.client
			.request("system_accountNextIndex", rpc_params![account])
			.await?;
		Ok(value)
	}

	pub async fn chain(&self) -> Result<String, subxt::Error> {
		let value: String = self.client.request("system_chain", rpc_params![]).await?;
		Ok(value)
	}

	pub async fn chain_type(&self) -> Result<String, subxt::Error> {
		let value: String = self
			.client
			.request("system_chainType", rpc_params![])
			.await?;
		Ok(value)
	}

	pub async fn health(&self) -> Result<SystemHealth, subxt::Error> {
		let value: SystemHealth = self.client.request("system_health", rpc_params![]).await?;
		Ok(value)
	}

	pub async fn local_listen_addresses(&self) -> Result<Vec<String>, subxt::Error> {
		let value: Vec<String> = self
			.client
			.request("system_localListenAddresses", rpc_params![])
			.await?;
		Ok(value)
	}

	pub async fn local_peer_id(&self) -> Result<String, subxt::Error> {
		let value: String = self
			.client
			.request("system_localPeerId", rpc_params![])
			.await?;
		Ok(value)
	}

	pub async fn name(&self) -> Result<String, subxt::Error> {
		let value: String = self.client.request("system_name", rpc_params![]).await?;
		Ok(value)
	}

	pub async fn node_roles(&self) -> Result<Vec<NodeRole>, subxt::Error> {
		let value: Vec<NodeRole> = self
			.client
			.request("system_nodeRoles", rpc_params![])
			.await?;
		Ok(value)
	}

	pub async fn peers(&self) -> Result<Vec<PeerInfo>, subxt::Error> {
		let value: Vec<PeerInfo> = self.client.request("system_peers", rpc_params![]).await?;
		Ok(value)
	}

	pub async fn properties(&self) -> Result<Properties, subxt::Error> {
		let value: Properties = self
			.client
			.request("system_properties", rpc_params![])
			.await?;
		Ok(value)
	}

	pub async fn sync_state(&self) -> Result<SyncState, subxt::Error> {
		let value: SyncState = self
			.client
			.request("system_syncState", rpc_params![])
			.await?;
		Ok(value)
	}

	pub async fn version(&self) -> Result<String, subxt::Error> {
		let value: String = self.client.request("system_version", rpc_params![]).await?;
		Ok(value)
	}
}

pub struct Chain {
	client: RpcClient,
}

impl Chain {
	pub fn new(client: RpcClient) -> Self {
		Self { client }
	}

	pub async fn get_block(
		&self,
		at: Option<BlockHash>,
	) -> Result<AvailBlockDetailsRPC, subxt::Error> {
		let value: AvailBlockDetailsRPC = self
			.client
			.request("chain_getBlock", rpc_params![at])
			.await?;
		Ok(value)
	}

	pub async fn get_block_hash(
		&self,
		block_number: Option<BlockNumber>,
	) -> Result<BlockHash, subxt::Error> {
		let value: BlockHash = self
			.client
			.request("chain_getBlockHash", rpc_params![block_number])
			.await?;
		Ok(value)
	}

	pub async fn get_finalized_head(&self) -> Result<BlockHash, subxt::Error> {
		let value: BlockHash = self
			.client
			.request("chain_getFinalizedHead", rpc_params![])
			.await?;
		Ok(value)
	}

	pub async fn get_header(&self, at: Option<BlockHash>) -> Result<AvailHeader, subxt::Error> {
		let value: AvailHeader = self
			.client
			.request("chain_getHeader", rpc_params![at])
			.await?;
		Ok(value)
	}
}

pub struct Author {
	client: RpcClient,
}

impl Author {
	pub fn new(client: RpcClient) -> Self {
		Self { client }
	}

	pub async fn rotate_keys(&self) -> Result<Vec<u8>, subxt::Error> {
		let bytes: Bytes = self
			.client
			.request("author_rotateKeys", rpc_params![])
			.await?;
		Ok(bytes.0)
	}
}

pub struct Kate {
	client: RpcClient,
}

impl Kate {
	pub fn new(client: RpcClient) -> Self {
		Self { client }
	}

	pub async fn block_length(&self, at: Option<BlockHash>) -> Result<BlockLength, subxt::Error> {
		let result: BlockLength = self
			.client
			.request("kate_blockLength", rpc_params![at])
			.await?;
		Ok(result)
	}

	pub async fn query_data_proof(
		&self,
		transaction_index: u32,
		at: Option<BlockHash>,
	) -> Result<ProofResponse, subxt::Error> {
		let result: ProofResponse = self
			.client
			.request("kate_queryDataProof", rpc_params![transaction_index, at])
			.await?;
		Ok(result)
	}

	pub async fn query_proof(
		&self,
		cells: Vec<Cell>,
		at: Option<BlockHash>,
	) -> Result<Vec<GDataProof>, subxt::Error> {
		let result: Vec<GDataProof> = self
			.client
			.request("kate_queryProof", rpc_params![cells, at])
			.await?;
		Ok(result)
	}

	pub async fn query_rows(
		&self,
		rows: Vec<u32>,
		at: Option<BlockHash>,
	) -> Result<Vec<GRow>, subxt::Error> {
		let result: Vec<GRow> = self
			.client
			.request("kate_queryRows", rpc_params![rows, at])
			.await?;
		Ok(result)
	}
}

#[rpc(client, namespace = "kate")]
pub trait KateRpc {
	#[method(name = "queryRows")]
	async fn query_rows(&self, rows: Rows, block: H256) -> RpcResult<Vec<GRow>>;

	#[method(name = "queryProof")]
	async fn query_proof(&self, cells: Cells, block: H256) -> RpcResult<Vec<GDataProof>>;

	#[method(name = "blockLength")]
	async fn query_block_length(&self, block: H256) -> RpcResult<BlockLength>;

	#[method(name = "queryDataProof")]
	async fn query_data_proof(&self, transaction_index: u32, at: H256) -> RpcResult<ProofResponse>;
}
