// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A collection of node-specific RPC methods.
//!
//! Since `substrate` core functionality makes no assumptions
//! about the modules used inside the runtime, so do
//! RPC methods defined in `sc-rpc` crate.
//! It means that `client/rpc` can't have any methods that
//! need some strong assumptions about the particular runtime.
//!
//! The RPCs available in this crate however can make some assumptions
//! about how the runtime is constructed and what FRAME pallets
//! are part of it. Therefore all node-runtime-specific RPCs can
//! be placed here or imported from corresponding FRAME RPC definitions.

//! # Data Availability Changes
//! - Add Kate RPC extension.
//! - Remove `sc_rpc::dev` extension.

#![warn(missing_docs)]

use std::sync::Arc;

use da_runtime::{
	apis::{DataAvailApi, KateApi, VectorApi},
	AccountId, Balance, BlockNumber, Hash, Index, NodeBlock as Block,
};
use jsonrpsee::RpcModule;
use sc_client_api::AuxStore;
use sc_consensus_babe::BabeWorkerHandle;
use sc_consensus_grandpa::{
	FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus::SelectChain;
use sp_consensus_babe::BabeApi;
use sp_keystore::KeystorePtr;

/// Extra dependencies for BABE.
pub struct BabeDeps {
	/// A handle to the BABE worker for issuing requests.
	pub babe_worker_handle: BabeWorkerHandle<Block>,
	/// The keystore that manages the keys of the node.
	pub keystore: KeystorePtr,
}

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B> {
	/// Voting round info.
	pub shared_voter_state: SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
	/// Receives notifications about justification events from Grandpa.
	pub justification_stream: GrandpaJustificationStream<Block>,
	/// Executor to drive the subscription manager in the Grandpa RPC handler.
	pub subscription_executor: SubscriptionTaskExecutor,
	/// Finality proof provider.
	pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Full client dependencies.
pub struct FullDeps<C, P, SC, B> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// The SelectChain Strategy
	pub select_chain: SC,
	/// A copy of the chain spec.
	pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// BABE specific dependencies.
	pub babe: BabeDeps,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,
	/// The maximum number of cells that can be requested in one go.
	pub kate_max_cells_size: usize,
	/// Enable Kate RPCs
	pub kate_rpc_enabled: bool,
	/// Enable Kate RPCs Metrics
	///
	/// Should not be used unless unless you know what you're doing.
	pub kate_rpc_metrics_enabled: bool,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, SC, B>(
	deps: FullDeps<C, P, SC, B>,
	backend: Arc<B>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>
		+ sc_client_api::BlockBackend<Block>
		+ HeaderBackend<Block>
		+ AuxStore
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ Sync
		+ Send
		+ 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: mmr_rpc::MmrRuntimeApi<Block, <Block as sp_runtime::traits::Block>::Hash, BlockNumber>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: BabeApi<Block>,
	C::Api: BlockBuilder<Block>,
	C::Api: DataAvailApi<Block> + KateApi<Block> + VectorApi<Block>,
	P: TransactionPool + 'static,
	SC: SelectChain<Block> + 'static,
	B: sc_client_api::Backend<Block> + Send + Sync + 'static,
	B::State: sc_client_api::backend::StateBackend<sp_runtime::traits::HashingFor<Block>>,
{
	#[cfg(feature = "grandpa-justifications")]
	use kate_rpc::justifications::{GrandpaJustifications, GrandpaServer};
	use kate_rpc::metrics::KateApiMetricsServer;
	use kate_rpc::{Kate, KateApiServer};
	use mmr_rpc::{Mmr, MmrApiServer};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use sc_consensus_babe_rpc::{Babe, BabeApiServer};
	use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
	use sc_rpc_spec_v2::chain_spec::{ChainSpec, ChainSpecApiServer};
	use sc_sync_state_rpc::{SyncState, SyncStateApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};
	use substrate_state_trie_migration_rpc::{StateMigration, StateMigrationApiServer};

	#[cfg(feature = "testing-environment")]
	use testing_rpc::{TestingApiServer, TestingEnv};

	let mut io = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		select_chain,
		chain_spec,
		deny_unsafe,
		babe,
		grandpa,
		kate_max_cells_size,
		kate_rpc_enabled,
		kate_rpc_metrics_enabled,
	} = deps;

	let BabeDeps {
		keystore,
		babe_worker_handle,
	} = babe;
	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;

	let is_dev_chain = chain_spec.id().ends_with("development_network");
	let chain_name = chain_spec.name().to_string();
	let genesis_hash = client
		.block_hash(0)
		.ok()
		.flatten()
		.expect("Genesis block exists; qed");
	let properties = chain_spec.properties();
	io.merge(ChainSpec::new(chain_name, genesis_hash, properties).into_rpc())?;

	io.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;
	// Making synchronous calls in light client freezes the browser currently,
	// more context: https://github.com/paritytech/substrate/pull/3480
	// These RPCs should use an asynchronous caller instead.
	io.merge(
		Mmr::new(
			client.clone(),
			backend
				.offchain_storage()
				.ok_or("Backend doesn't provide an offchain storage")?,
		)
		.into_rpc(),
	)?;
	io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	io.merge(
		Babe::new(
			client.clone(),
			babe_worker_handle.clone(),
			keystore,
			select_chain,
			deny_unsafe,
		)
		.into_rpc(),
	)?;
	io.merge(
		Grandpa::new(
			subscription_executor,
			shared_authority_set.clone(),
			shared_voter_state,
			justification_stream,
			finality_provider,
		)
		.into_rpc(),
	)?;

	io.merge(
		SyncState::new(
			chain_spec,
			client.clone(),
			shared_authority_set,
			babe_worker_handle,
		)?
		.into_rpc(),
	)?;

	io.merge(StateMigration::new(client.clone(), backend, deny_unsafe).into_rpc())?;

	if is_dev_chain || kate_rpc_metrics_enabled {
		io.merge(KateApiMetricsServer::into_rpc(Kate::<C, Block>::new(
			client.clone(),
			kate_max_cells_size,
		)))?;
	}

	if is_dev_chain || kate_rpc_enabled || kate_rpc_metrics_enabled {
		io.merge(KateApiServer::into_rpc(Kate::<C, Block>::new(
			client.clone(),
			kate_max_cells_size,
		)))?;
	}

	#[cfg(feature = "testing-environment")]
	io.merge(TestingApiServer::into_rpc(TestingEnv))?;

	#[cfg(feature = "grandpa-justifications")]
	io.merge(GrandpaServer::into_rpc(
		GrandpaJustifications::<C, Block>::new(client),
	))?;

	Ok(io)
}
