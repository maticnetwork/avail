// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

use std::sync::Arc;

use avail_node::chains;
use da_runtime::Block;
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use sc_cli::{Result, SubstrateCli};
use sc_service::PartialComponents;
#[cfg(feature = "try-runtime")]
use {
	crate::service::ExecutorDispatch, da_runtime::constants::time::SLOT_DURATION,
	try_runtime_cli::block_building_info::substrate_info,
};

use crate::{
	cli::{Cli, Subcommand},
	service::{self, new_partial, FullClient},
};

use avail_node::NODE_VERSION;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Avail Node".into()
	}

	fn impl_version() -> String {
		let commit_hash = env!("SUBSTRATE_CLI_COMMIT_HASH");
		format!("{NODE_VERSION}-{commit_hash}")
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/availproject/avail/issues/new/choose".into()
	}

	fn copyright_start_year() -> i32 {
		// Notice
		2017
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		let spec = match id {
			"" => {
				return Err(
					"Please specify which chain you want to run, e.g. --chain mainnet".into(),
				)
			},
			"dev" => Box::new(chains::dev::chain_spec()),
			"dev.tri" => Box::new(chains::dev_tri::chain_spec()),
			"devnet0" => Box::new(chains::devnet0::chain_spec()?),
			"mainnet" => Box::new(chains::mainnet::chain_spec()?),
			"turing" => Box::new(chains::turing::chain_spec()?),
			path => Box::new(chains::ChainSpec::from_json_file(
				std::path::PathBuf::from(path),
			)?),
		};
		Ok(spec)
	}
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				service::new_full(config, cli).map_err(sc_cli::Error::Service)
			})
		},
		/*Some(Subcommand::Inspect(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| cmd.run::<Block, RuntimeApi, ExecutorDispatch>(config))
		},*/
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				// This switch needs to be in the client, since the client decides
				// which sub-commands it wants to support.
				match cmd {
					BenchmarkCmd::Pallet(cmd) => {
						if !cfg!(feature = "runtime-benchmarks") {
							return Err(
								"Runtime benchmarking wasn't enabled when building the node. \
							You can enable it with `--features runtime-benchmarks`."
									.into(),
							);
						}
						cmd.run::<Block, (
							frame_system::native::hosted_header_builder::hosted_header_builder::HostFunctions,
							avail_base::mem_tmp_storage::hosted_mem_tmp_storage::HostFunctions,
							da_runtime::kate::native::hosted_kate::HostFunctions,
						)>(config)
					},
					BenchmarkCmd::Block(_cmd) => {
						unimplemented!();
						/*
						// ensure that we keep the task manager alive
						let partial = new_partial(&config, cli.unsafe_da_sync)?;
						cmd.run(partial.client)
						*/
					},
					#[cfg(not(feature = "runtime-benchmarks"))]
					BenchmarkCmd::Storage(_) => Err(
						"Storage benchmarking can be enabled with `--features runtime-benchmarks`."
							.into(),
					),
					#[cfg(feature = "runtime-benchmarks")]
					BenchmarkCmd::Storage(_cmd) => {
						unimplemented!();
						/*
						// ensure that we keep the task manager alive
						let partial = new_partial(&config, cli.unsafe_da_sync)?;
						let db = partial.backend.expose_db();
						let storage = partial.backend.expose_storage();

						cmd.run(config, partial.client, db, storage)
						*/
					},
					BenchmarkCmd::Overhead(_cmd) => {
						unimplemented!();
						/*
						// ensure that we keep the task manager alive
						let partial = new_partial(&config, cli.unsafe_da_sync)?;
						let ext_builder = RemarkBuilder::new(partial.client.clone());

						cmd.run(
							config,
							partial.client,
							inherent_benchmark_data()?,
							Vec::new(),
							&ext_builder,
						)
						*/
					},
					BenchmarkCmd::Extrinsic(_cmd) => {
						unimplemented!();
						/*
						// ensure that we keep the task manager alive
						let partial = service::new_partial(&config, cli.unsafe_da_sync)?;
						// Register the *Remark* and *TKA* builders.
						let ext_factory = ExtrinsicFactory(vec![
							Box::new(RemarkBuilder::new(partial.client.clone())),
							Box::new(TransferKeepAliveBuilder::new(
								partial.client.clone(),
								Sr25519Keyring::Alice.to_account_id(),
								ExistentialDeposit::get(),
							)),
						]);

						cmd.run(
							partial.client,
							inherent_benchmark_data()?,
							Vec::new(),
							&ext_factory,
						)*/
					},
					BenchmarkCmd::Machine(cmd) => {
						cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())
					},
				}
			})
		},
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					import_queue,
					..
				} = new_partial(&config, cli.partial_clone())?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					..
				} = new_partial(&config, cli.partial_clone())?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					..
				} = new_partial(&config, cli.partial_clone())?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					import_queue,
					..
				} = new_partial(&config, cli.partial_clone())?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					backend,
					..
				} = new_partial(&config, cli.partial_clone())?;
				let aux_revert = Box::new(|client: Arc<FullClient>, backend, blocks| {
					sc_consensus_babe::revert(client.clone(), backend, blocks)?;
					sc_consensus_grandpa::revert(client, blocks)?;
					Ok(())
				});
				Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
			})
		},
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			use sc_executor::{sp_wasm_interface::ExtendedHostFunctions, NativeExecutionDispatch};
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				// we don't need any of the components of new_partial, just a runtime, or a task
				// manager to do `async_run`.
				let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
				let task_manager =
					sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
						.map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;
				let info_provider = substrate_info(SLOT_DURATION);
				Ok((
					cmd.run::<Block, ExtendedHostFunctions<
						sp_io::SubstrateHostFunctions,
						<ExecutorDispatch as NativeExecutionDispatch>::ExtendHostFunctions,
					>, _>(Some(info_provider)),
					task_manager,
				))
			})
		},
		#[cfg(not(feature = "try-runtime"))]
		Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
				You can enable it with `--features try-runtime`."
			.into()),
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<Block>(&config))
		},
	}
}
