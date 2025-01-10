// This file is part of Substrate.

// Copyright (C) 2018-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.
#![allow(dead_code)]

use crate::{cli::Cli, rpc as node_rpc};
use avail_core::AppId;
use da_runtime::{apis::RuntimeApi, NodeBlock as Block, Runtime};

use codec::Encode;
use frame_system_rpc_runtime_api::AccountNonceApi;
use futures::prelude::*;
use pallet_transaction_payment::ChargeTransactionPayment;
use sc_client_api::{Backend, BlockBackend};
use sc_consensus_babe::{self, SlotProportion};
pub use sc_executor::NativeElseWasmExecutor;
use sc_network::{Event, NetworkEventStream, NetworkService};
use sc_network_sync::SyncingService;
use sc_service::{
	error::Error as ServiceError, Configuration, RpcHandlers, TaskManager, WarpSyncParams,
};
use sc_telemetry::custom_telemetry::external::BlockIntervalFromNode;
use sc_telemetry::{custom_telemetry::CustomTelemetryWorker, Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ProvideRuntimeApi;
use sp_core::crypto::Pair;
use sp_runtime::{generic::Era, traits::Block as BlockT, SaturatedConversion};
use std::{path::Path, sync::Arc};
use substrate_prometheus_endpoint::{PrometheusError, Registry};

pub const LOG_TARGET: &str = "avail::node::service";

// Declare an instance of the native executor named `ExecutorDispatch`. Include the wasm binary as
// the equivalent wasm code.
pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
	type ExtendHostFunctions = (
		frame_benchmarking::benchmarking::HostFunctions,
		frame_system::native::hosted_header_builder::hosted_header_builder::HostFunctions,
		avail_base::mem_tmp_storage::hosted_mem_tmp_storage::HostFunctions,
		da_runtime::kate::native::hosted_kate::HostFunctions,
	);

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		da_runtime::apis::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		da_runtime::native_version()
	}
}

/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

/// The full client type definition.
pub type FullClient =
	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;
pub type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport =
	sc_consensus_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;

/// The transaction pool type definition.
pub type TransactionPool = sc_transaction_pool::FullPool<Block, FullClient>;

pub type BlockImport = crate::da_block_import::BlockImport<
	Block,
	FullClient,
	sc_consensus_babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>,
>;

/// Fetch the nonce of the given `account` from the chain state.
///
/// Note: Should only be used for tests.
pub fn fetch_nonce(client: &FullClient, account: sp_core::sr25519::Pair) -> u32 {
	let best_hash = client.chain_info().best_hash;
	client
		.runtime_api()
		.account_nonce(best_hash, account.public().into())
		.expect("Fetching account nonce works; qed")
}

/// Create a transaction using the given `call`.
///
/// The transaction will be signed by `sender`. If `nonce` is `None` it will be fetched from the
/// state of the best block.
///
/// Note: Should only be used for tests.
pub fn create_extrinsic(
	client: &FullClient,
	sender: sp_core::sr25519::Pair,
	function: impl Into<da_runtime::RuntimeCall>,
	nonce: Option<u32>,
	app_id: AppId,
) -> da_runtime::UncheckedExtrinsic {
	let function = function.into();
	let genesis_hash = client
		.block_hash(0)
		.ok()
		.flatten()
		.expect("Genesis block exists; qed");
	let best_hash = client.chain_info().best_hash;
	let best_block = client.chain_info().best_number;
	let nonce = nonce.unwrap_or_else(|| fetch_nonce(client, sender.clone()));

	let period = da_runtime::BlockHashCount::get()
		.checked_next_power_of_two()
		.map(|c| c / 2)
		.unwrap_or(2) as u64;
	let tip = 0;
	let extra = (
		frame_system::CheckNonZeroSender::<Runtime>::new(),
		frame_system::CheckSpecVersion::<Runtime>::new(),
		frame_system::CheckTxVersion::<Runtime>::new(),
		frame_system::CheckGenesis::<Runtime>::new(),
		frame_system::CheckEra::<Runtime>::from(Era::mortal(period, best_block.saturated_into())),
		frame_system::CheckNonce::<Runtime>::from(nonce),
		frame_system::CheckWeight::<Runtime>::new(),
		ChargeTransactionPayment::<Runtime>::from(tip),
		da_control::CheckAppId::<Runtime>::from(app_id),
	);

	let raw_payload = da_runtime::SignedPayload::from_raw(
		function.clone(),
		extra.clone(),
		(
			(),
			da_runtime::VERSION.spec_version,
			da_runtime::VERSION.transaction_version,
			genesis_hash,
			best_hash,
			(),
			(),
			(),
			(),
		),
	);
	let signature = raw_payload.using_encoded(|e| sender.sign(e));

	da_runtime::UncheckedExtrinsic::new_signed(
		function,
		sp_runtime::AccountId32::from(sender.public()).into(),
		da_runtime::Signature::Sr25519(signature),
		extra,
	)
}

/// Creates a new partial node.
#[allow(clippy::redundant_clone)]
#[allow(clippy::type_complexity)]
pub fn new_partial(
	config: &Configuration,
	cli: Cli,
) -> Result<
	sc_service::PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(
			impl Fn(
				node_rpc::DenyUnsafe,
				sc_rpc::SubscriptionTaskExecutor,
			) -> Result<jsonrpsee::RpcModule<()>, sc_service::Error>,
			(
				BlockImport,
				sc_consensus_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
				sc_consensus_babe::BabeLink<Block>,
			),
			sc_consensus_grandpa::SharedVoterState,
			Option<Telemetry>,
		),
	>,
	ServiceError,
> {
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = sc_service::new_native_or_wasm_executor(config);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager
			.spawn_handle()
			.spawn("telemetry", None, worker.run());
		telemetry
	});

	let telemetry_handle = telemetry.as_ref().map(|t| t.handle());
	let custom_telemetry_worker = CustomTelemetryWorker {
		handle: telemetry_handle,
		sampling_interval_ms: 5_000u128,
		max_interval_buffer_size: 5,
		max_block_request_buffer_size: 10,
		is_authority: config.role.is_authority(),
	};
	task_manager.spawn_handle().spawn(
		"custom_telemetry",
		None,
		custom_telemetry_worker.run(Some(filter_intervals), None),
	);

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	#[cfg(not(feature = "grandpa-justifications"))]
	let grandpa_justification_period = GRANDPA_JUSTIFICATION_PERIOD;
	#[cfg(feature = "grandpa-justifications")]
	let grandpa_justification_period = cli.grandpa_justification_period;

	let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		grandpa_justification_period,
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;
	let justification_import = grandpa_block_import.clone();

	let (block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::configuration(&*client)?,
		grandpa_block_import,
		client.clone(),
	)?;

	let da_block_import = BlockImport::new(client.clone(), block_import, cli.unsafe_da_sync);

	let slot_duration = babe_link.config().slot_duration();
	let (import_queue, babe_worker_handle) =
		sc_consensus_babe::import_queue(sc_consensus_babe::ImportQueueParams {
			link: babe_link.clone(),
			block_import: da_block_import.clone(),
			justification_import: Some(Box::new(justification_import)),
			client: client.clone(),
			select_chain: select_chain.clone(),
			create_inherent_data_providers: move |_, ()| async move {
				let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

				let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);

				Ok((slot, timestamp))
			},
			spawner: &task_manager.spawn_essential_handle(),
			registry: config.prometheus_registry(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
		})?;

	let import_setup = (da_block_import, grandpa_link, babe_link);

	let (rpc_extensions_builder, rpc_setup) = {
		let (_, grandpa_link, _) = &import_setup;

		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let shared_voter_state = sc_consensus_grandpa::SharedVoterState::empty();
		let shared_voter_state2 = shared_voter_state.clone();

		let finality_proof_provider = sc_consensus_grandpa::FinalityProofProvider::new_for_service(
			backend.clone(),
			Some(shared_authority_set.clone()),
		);

		let client = client.clone();
		let pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let keystore = keystore_container.keystore();
		let chain_spec = config.chain_spec.cloned_box();

		let rpc_backend = backend.clone();
		let rpc_extensions_builder = move |deny_unsafe, subscription_executor| {
			let deps = node_rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				select_chain: select_chain.clone(),
				chain_spec: chain_spec.cloned_box(),
				deny_unsafe,
				babe: node_rpc::BabeDeps {
					keystore: keystore.clone(),
					babe_worker_handle: babe_worker_handle.clone(),
				},
				grandpa: node_rpc::GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
					justification_stream: justification_stream.clone(),
					subscription_executor,
					finality_provider: finality_proof_provider.clone(),
				},
				kate_max_cells_size: cli.kate_max_cells_size,
				kate_rpc_enabled: cli.kate_rpc_enabled,
				kate_rpc_metrics_enabled: cli.kate_rpc_metrics_enabled,
			};

			node_rpc::create_full(deps, rpc_backend.clone()).map_err(Into::into)
		};

		(rpc_extensions_builder, shared_voter_state2)
	};

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		other: (rpc_extensions_builder, import_setup, rpc_setup, telemetry),
	})
}

/// Result of [`new_full_base`].
pub struct NewFullBase {
	/// The task manager of the node.
	pub task_manager: TaskManager,
	/// The client instance of the node.
	pub client: Arc<FullClient>,
	/// The networking service of the node.
	pub network: Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
	/// The syncing service of the node.
	pub sync: Arc<SyncingService<Block>>,
	/// The transaction pool of the node.
	pub transaction_pool: Arc<TransactionPool>,
	/// The rpc handlers of the node.
	pub rpc_handlers: RpcHandlers,
}

/// Creates a full service from the configuration.
#[allow(clippy::too_many_arguments)]
pub fn new_full_base(
	config: Configuration,
	disable_hardware_benchmarks: bool,
	with_startup_data: impl FnOnce(&BlockImport, &sc_consensus_babe::BabeLink<Block>),
	cli: Cli,
) -> Result<NewFullBase, ServiceError> {
	let hwbench = if !disable_hardware_benchmarks {
		config.database.path().map(|database_path| {
			let _ = std::fs::create_dir_all(database_path);
			sc_sysinfo::gather_hwbench(Some(database_path))
		})
	} else {
		None
	};

	#[cfg(not(feature = "grandpa-justifications"))]
	let grandpa_justification_period = GRANDPA_JUSTIFICATION_PERIOD;
	#[cfg(feature = "grandpa-justifications")]
	let grandpa_justification_period = cli.grandpa_justification_period;

	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (rpc_builder, import_setup, rpc_setup, mut telemetry),
	} = new_partial(&config, cli)?;

	let shared_voter_state = rpc_setup;
	let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;
	let mut net_config = sc_network::config::FullNetworkConfiguration::new(&config.network);
	let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(
		&client
			.block_hash(0)
			.ok()
			.flatten()
			.expect("Genesis block exists; qed"),
		&config.chain_spec,
	);
	let (grandpa_protocol_config, grandpa_notification_service) =
		sc_consensus_grandpa::grandpa_peers_set_config(grandpa_protocol_name.clone());
	net_config.add_notification_protocol(grandpa_protocol_config);
	let warp_sync = Arc::new(sc_consensus_grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		import_setup.1.shared_authority_set().clone(),
		Vec::default(),
	));

	let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_params: Some(WarpSyncParams::WithProvider(warp_sync)),
			block_relay: None,
		})?;

	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks =
		Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();
	let enable_offchain_worker = config.offchain_worker.enabled;
	if let Some(reg) = prometheus_registry.as_ref() {
		extend_metrics(reg)?;
	}

	let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config,
		backend: backend.clone(),
		client: client.clone(),
		keystore: keystore_container.keystore(),
		network: network.clone(),
		rpc_builder: Box::new(rpc_builder),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		system_rpc_tx,
		tx_handler_controller,
		sync_service: sync_service.clone(),
		telemetry: telemetry.as_mut(),
	})?;

	if let Some(hwbench) = hwbench {
		sc_sysinfo::print_hwbench(&hwbench);

		if let Some(ref mut telemetry) = telemetry {
			let telemetry_handle = telemetry.handle();
			task_manager.spawn_handle().spawn(
				"telemetry_hwbench",
				None,
				sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
			);
		}
	}

	let (block_import, grandpa_link, babe_link) = import_setup;

	(with_startup_data)(&block_import, &babe_link);

	if let sc_service::config::Role::Authority { .. } = &role {
		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let client_clone = client.clone();
		let slot_duration = babe_link.config().slot_duration();
		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore_container.keystore(),
			client: client.clone(),
			select_chain,
			env: proposer,
			block_import,
			sync_oracle: sync_service.clone(),
			justification_sync_link: sync_service.clone(),
			create_inherent_data_providers: move |parent, ()| {
				let client_clone = client_clone.clone();
				async move {
					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

					let storage_proof =
						sp_transaction_storage_proof::registration::new_data_provider(
							&*client_clone,
							&parent,
						)?;

					Ok((slot, timestamp, storage_proof))
				}
			},
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			block_proposal_slot_portion: SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		let babe = sc_consensus_babe::start_babe(babe_config)?;
		task_manager.spawn_essential_handle().spawn_blocking(
			"babe-proposer",
			Some("block-authoring"),
			babe,
		);
	}

	// Spawn authority discovery module.
	if role.is_authority() {
		let authority_discovery_role =
			sc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore());
		let dht_event_stream =
			network
				.event_stream("authority-discovery")
				.filter_map(|e| async move {
					match e {
						Event::Dht(e) => Some(e),
						_ => None,
					}
				});
		let (authority_discovery_worker, _service) =
			sc_authority_discovery::new_worker_and_service_with_config(
				sc_authority_discovery::WorkerConfig {
					publish_non_global_ips: auth_disc_publish_non_global_ips,
					..Default::default()
				},
				client.clone(),
				network.clone(),
				Box::pin(dht_event_stream),
				authority_discovery_role,
				prometheus_registry.clone(),
			);

		task_manager.spawn_handle().spawn(
			"authority-discovery-worker",
			Some("networking"),
			authority_discovery_worker.run(),
		);
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore = if role.is_authority() {
		Some(keystore_container.keystore())
	} else {
		None
	};

	let grandpa_config = sc_consensus_grandpa::Config {
		// Considering our block_time of 20 seconds, we may consider increasing this to 2 seconds without introducing delay in finality.
		gossip_duration: std::time::Duration::from_millis(1000),
		justification_generation_period: grandpa_justification_period,
		name: Some(name),
		observer_enabled: false,
		keystore,
		local_role: role.clone(),
		telemetry: telemetry.as_ref().map(|x| x.handle()),
		protocol_name: grandpa_protocol_name,
	};

	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_config = sc_consensus_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network: network.clone(),
			notification_service: grandpa_notification_service,
			sync: Arc::new(sync_service.clone()),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state,
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			sc_consensus_grandpa::run_grandpa_voter(grandpa_config)?,
		);
	}

	if enable_offchain_worker {
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-work",
			sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				keystore: Some(keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: network.clone(),
				is_validator: role.is_authority(),
				enable_http_requests: true,
				custom_extensions: move |_| vec![],
			})
			.run(client.clone(), task_manager.spawn_handle())
			.boxed(),
		);
	}

	network_starter.start_network();
	Ok(NewFullBase {
		task_manager,
		client,
		network,
		sync: sync_service,
		transaction_pool,
		rpc_handlers,
	})
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration, cli: Cli) -> Result<TaskManager, ServiceError> {
	let database_path = config.database.path().map(Path::to_path_buf);
	let storage_param = cli.storage_monitor.clone();
	let task_manager = new_full_base(
		config,
		cli.no_hardware_benchmarks,
		|_, _| (),
		cli,
		// cli.unsafe_da_sync,
		// cli.kate_max_cells_size,
		// cli.kate_rpc_enabled,
		// cli.kate_rpc_metrics_enabled,
	)
	.map(|NewFullBase { task_manager, .. }| task_manager)?;

	if let Some(database_path) = database_path {
		sc_storage_monitor::StorageMonitorService::try_spawn(
			storage_param,
			database_path,
			&task_manager.spawn_essential_handle(),
		)
		.map_err(|e| ServiceError::Application(e.into()))?;
	}

	Ok(task_manager)
}

fn extend_metrics(prometheus: &Registry) -> Result<(), PrometheusError> {
	use avail_base::metrics::{AvailMetrics, AVAIL_METRICS};

	AVAIL_METRICS.get_or_try_init(|| AvailMetrics::new(prometheus))?;
	Ok(())
}

fn filter_intervals(intervals: Vec<BlockIntervalFromNode>) -> Vec<BlockIntervalFromNode> {
	// We are working on a 7.5 second telemetry interval basis.
	// If there are more than 4 blocks worth of events then the node is still syncing and we
	// don't want to broadcast sync related data.
	if intervals.len() > 4 {
		return vec![];
	}

	intervals
}

/*
#[cfg(test)]
mod tests {
	use crate::service::{new_full_base, NewFullBase};
	use codec::Encode;
	use da_runtime::{
		currency::CENTS, Address, BalancesCall, Block, Call, DigestItem, Signature,
		UncheckedExtrinsic, SLOT_DURATION,
	};
	use sc_client_api::BlockBackend;
	use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
	use sc_consensus_babe::{BabeIntermediate, CompatibleDigestItem, INTERMEDIATE_KEY};
	use sc_consensus_epochs::descendent_query;
	use sc_keystore::LocalKeystore;
	use sc_transaction_pool_api::{ChainEvent, MaintainedTransactionPool};
	use sp_consensus::{BlockOrigin, Environment, Proposer};
	use sp_core::{crypto::Pair as CryptoPair, Public};
	use sp_inherents::InherentDataProvider;
	use sp_keyring::AccountKeyring;
	use sp_keystore::{Keystore, KeystorePtr};
	use sp_runtime::{
		generic::{BlockId, Digest, Era, SignedPayload},
		key_types::BABE,
		traits::{Block as BlockT, Header as HeaderT, IdentifyAccount, Verify},
		RuntimeAppPublic,
	};
	use sp_timestamp;

	use crate::service::{new_full_base, NewFullBase};

	type AccountPublic = <Signature as Verify>::Signer;

	#[test]
	// It is "ignored", but the node-cli ignored tests are running on the CI.
	// This can be run locally with `cargo test --release -p node-cli test_sync -- --ignored`.
	#[ignore]
	fn test_sync() {
		sp_tracing::try_init_simple();

		let keystore_path = tempfile::tempdir().expect("Creates keystore path");
		let keystore: KeystorePtr =
			Arc::new(LocalKeystore::open(keystore_path.path(), None).expect("Creates keystore"));
		let alice: sp_consensus_babe::AuthorityId =
			Keystore::sr25519_generate_new(&*keystore, BABE, Some("//Alice"))
				.expect("Creates authority pair")
				.into();

		let chain_spec = crate::chain_spec::tests::integration_test_config_with_single_authority();

		// For the block factory
		let mut slot = 1u64;

		// For the extrinsics factory
		let bob = Arc::new(AccountKeyring::Bob.pair());
		let charlie = Arc::new(AccountKeyring::Charlie.pair());
		let mut index = 0;

		sc_service_test::sync(
			chain_spec,
			|config| {
				let mut setup_handles = None;
				let NewFullBase {
					task_manager,
					client,
					network,
					transaction_pool,
					..
				} = new_full_base(
					config,
					|block_import: &sc_consensus_babe::BabeBlockImport<Block, _, _>,
					 babe_link: &sc_consensus_babe::BabeLink<Block>| {
						setup_handles = Some((block_import.clone(), babe_link.clone()));
					},
					false,
				)?;

				let node = sc_service_test::TestNetComponents::new(
					task_manager,
					client,
					network,
					transaction_pool,
				);
				Ok((node, setup_handles.unwrap()))
			},
			|service, &mut (ref mut block_import, ref babe_link)| {
				let parent_id = BlockId::number(service.client().chain_info().best_number);
				let parent_header = service.client().header(&parent_id).unwrap().unwrap();
				let parent_hash = parent_header.hash();
				let parent_number = *parent_header.number();

				futures::executor::block_on(service.transaction_pool().maintain(
					ChainEvent::NewBestBlock {
						hash: parent_header.hash(),
						tree_route: None,
					},
				));

				let mut proposer_factory = sc_basic_authorship::ProposerFactory::new(
					service.spawn_handle(),
					service.client(),
					service.transaction_pool(),
					None,
					None,
				);

				let mut digest = Digest::default();

				// even though there's only one authority some slots might be empty,
				// so we must keep trying the next slots until we can claim one.
				let (babe_pre_digest, epoch_descriptor) = loop {
					let epoch_descriptor = babe_link
						.epoch_changes()
						.shared_data()
						.epoch_descriptor_for_child_of(
							descendent_query(&*service.client()),
							&parent_hash,
							parent_number,
							slot.into(),
						)
						.unwrap()
						.unwrap();

					let epoch = babe_link
						.epoch_changes()
						.shared_data()
						.epoch_data(&epoch_descriptor, |slot| {
							sc_consensus_babe::Epoch::genesis(&babe_link.config(), slot)
						})
						.unwrap();

					if let Some(babe_pre_digest) =
						sc_consensus_babe::authorship::claim_slot(slot.into(), &epoch, &keystore)
							.map(|(digest, _)| digest)
					{
						break (babe_pre_digest, epoch_descriptor);
					}

					slot += 1;
				};

				let inherent_data = (
					sp_timestamp::InherentDataProvider::new(
						std::time::Duration::from_millis(SLOT_DURATION * slot).into(),
					),
					sp_consensus_babe::inherents::InherentDataProvider::new(slot.into()),
				)
					.create_inherent_data()
					.expect("Creates inherent data");

				digest.push(<DigestItem as CompatibleDigestItem>::babe_pre_digest(
					babe_pre_digest,
				));

				let new_block = futures::executor::block_on(async move {
					let proposer = proposer_factory.init(&parent_header).await;
					proposer
						.unwrap()
						.propose(
							inherent_data,
							digest,
							std::time::Duration::from_secs(1),
							None,
						)
						.await
				})
				.expect("Error making test block")
				.block;

				let (new_header, new_body) = new_block.deconstruct();
				let pre_hash = new_header.hash();
				// sign the pre-sealed hash of the block and then
				// add it to a digest item.
				let to_sign = pre_hash.encode();
				let signature = Keystore::sign_with(
					&*keystore,
					sp_consensus_babe::AuthorityId::ID,
					&alice.to_public_crypto_pair(),
					&to_sign,
				)
				.unwrap()
				.unwrap()
				.try_into()
				.unwrap();
				let item = <DigestItem as CompatibleDigestItem>::babe_seal(signature);
				slot += 1;

				let mut params = BlockImportParams::new(BlockOrigin::File, new_header);
				params.post_digests.push(item);
				params.body = Some(new_body);
				params.intermediates.insert(
					Cow::from(INTERMEDIATE_KEY),
					Box::new(BabeIntermediate::<Block> { epoch_descriptor }) as Box<_>,
				);
				params.fork_choice = Some(ForkChoiceStrategy::LongestChain);

				futures::executor::block_on(block_import.import_block(params, Default::default()))
					.expect("error importing test block");
			},
			|service, _| {
				let amount = 5 * CENTS;
				let to: Address = AccountPublic::from(bob.public()).into_account().into();
				let from: Address = AccountPublic::from(charlie.public()).into_account().into();
				let genesis_hash = service.client().block_hash(0).unwrap().unwrap();
				let best_block_id = BlockId::number(service.client().chain_info().best_number);
				let (spec_version, transaction_version) = {
					let version = service.client().runtime_version_at(&best_block_id).unwrap();
					(version.spec_version, version.transaction_version)
				};
				let signer = charlie.clone();

				let function = Call::Balances(BalancesCall::transfer {
					dest: to.into(),
					value: amount,
				});

				let check_spec_version = frame_system::CheckSpecVersion::new();
				let check_tx_version = frame_system::CheckTxVersion::new();
				let check_genesis = frame_system::CheckGenesis::new();
				let check_era = frame_system::CheckEra::from(Era::Immortal);
				let check_nonce = frame_system::CheckNonce::from(index);
				let check_weight = frame_system::CheckWeight::new();
				let tx_payment = pallet_asset_tx_payment::ChargeAssetTxPayment::from(0, None);
				let extra = (
					check_spec_version,
					check_tx_version,
					check_genesis,
					check_era,
					check_nonce,
					check_weight,
					tx_payment,
				);
				let raw_payload = SignedPayload::from_raw(
					function,
					extra,
					(
						spec_version,
						transaction_version,
						genesis_hash,
						genesis_hash,
						(),
						(),
						(),
					),
				);
				let signature = raw_payload.using_encoded(|payload| signer.sign(payload));
				let (function, extra, _) = raw_payload.deconstruct();
				index += 1;
				UncheckedExtrinsic::new_signed(function, from.into(), signature.into(), extra)
					.into()
			},
		);
	}

	#[test]
	#[ignore]
	fn test_consensus() {
		sp_tracing::try_init_simple();

		sc_service_test::consensus(
			crate::chain_spec::tests::integration_test_config_with_two_authorities(),
			|config| {
				let NewFullBase {
					task_manager,
					client,
					network,
					transaction_pool,
					..
				} = new_full_base(config, |_, _| (), false)?;
				Ok(sc_service_test::TestNetComponents::new(
					task_manager,
					client,
					network,
					transaction_pool,
				))
			},
			vec!["//Alice".into(), "//Bob".into()],
		)
	}
}
*/
