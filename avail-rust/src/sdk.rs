use std::str::FromStr;

use subxt::backend::rpc::RpcClient;
use subxt_signer::{sr25519::Keypair, SecretUri};

use crate::{error::ClientError, rpcs::Rpc, transactions::Transactions, AOnlineClient};

#[derive(Clone)]
pub struct SDK {
	pub online_client: AOnlineClient,
	pub rpc_client: RpcClient,
	pub tx: Transactions,
	pub rpc: Rpc,
}

impl SDK {
	pub async fn new(endpoint: &str) -> Result<Self, ClientError> {
		Self::new_inner(endpoint, true).await
	}

	pub async fn new_insecure(endpoint: &str) -> Result<Self, ClientError> {
		Self::new_inner(endpoint, false).await
	}

	async fn new_inner(endpoint: &str, secure: bool) -> Result<Self, ClientError> {
		let (online_client, rpc_client) = initialize_api(endpoint, secure).await?;

		let rpc = Rpc::new(rpc_client.clone()).await;
		let tx = Transactions::new(online_client.clone(), rpc_client.clone());

		Ok(SDK {
			online_client,
			rpc_client,
			tx,
			rpc,
		})
	}

	pub fn alice() -> Result<Keypair, String> {
		let secret_uri = SecretUri::from_str("//Alice").map_err(|e| e.to_string())?;
		Keypair::from_uri(&secret_uri).map_err(|e| e.to_string())
	}

	pub fn one_avail() -> u128 {
		1_000_000_000_000_000_000u128
	}

	pub fn local_endpoint() -> &'static str {
		"ws://127.0.0.1:9944"
	}
}

pub async fn initialize_api(
	endpoint: &str,
	secure: bool,
) -> Result<(AOnlineClient, RpcClient), ClientError> {
	let rpc_client: RpcClient = match secure {
		true => RpcClient::from_url(endpoint).await?,
		false => RpcClient::from_insecure_url(endpoint).await?,
	};

	// Cloning RpcClient is cheaper and doesn't create a new WS connection
	let api = AOnlineClient::from_rpc_client(rpc_client.clone()).await?;

	Ok((api, rpc_client))
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WaitFor {
	BlockInclusion,
	BlockFinalization,
}
