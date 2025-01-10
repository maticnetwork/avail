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

/// An overarching CLI command definition.
#[derive(Debug, clap::Parser)]
pub struct Cli {
	/// Possible subcommand with parameters.
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub run: sc_cli::RunCmd,

	/// Disable automatic hardware benchmarks.
	///
	/// By default these benchmarks are automatically ran at startup and measure
	/// the CPU speed, the memory bandwidth and the disk speed.
	///
	/// The results are then printed out in the logs, and also sent as part of
	/// telemetry, if telemetry is enabled.
	#[arg(long)]
	pub no_hardware_benchmarks: bool,

	/// Disable checking commitment on imported block during sync
	#[arg(long, conflicts_with_all = &["validator"])]
	pub unsafe_da_sync: bool,

	/// Provides storage monitoring options on the node
	#[clap(flatten)]
	pub storage_monitor: sc_storage_monitor::StorageMonitorParams,

	/// Enable Kate RPC
	#[clap(long = "enable-kate-rpc", default_value_t = false)]
	pub kate_rpc_enabled: bool,

	/// Enable Kate RPC Metrics
	#[clap(long = "enable-kate-rpc-metrics", default_value_t = false)]
	pub kate_rpc_metrics_enabled: bool,

	/// The maximum number of cells that can be requested in one go.
	///
	/// Max size cannot exceed 10_000
	#[arg(long, default_value_t = 64, value_parser=kate_max_cells_size_upper_bound)]
	pub kate_max_cells_size: usize,

	/// The interval, in blocks, at which Grandpa justifications are either imported or generated and stored in the backend.
	#[cfg(feature = "grandpa-justifications")]
	#[arg(long, default_value_t =512, value_parser=grandpa_justification_period_bounds)]
	pub grandpa_justification_period: u32,
}

fn kate_max_cells_size_upper_bound(s: &str) -> Result<usize, String> {
	clap_num::number_range(s, 0, 10_000)
}

#[cfg(feature = "grandpa-justifications")]
fn grandpa_justification_period_bounds(s: &str) -> Result<u32, String> {
	clap_num::number_range(s, 1, u32::MAX)
}

/// Possible subcommands of the main binary.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Sub-commands concerned with benchmarking.
	/// The pallet benchmarking moved to the `pallet` sub-command.
	#[command(subcommand)]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),

	/// Try some command against runtime state.
	#[cfg(feature = "try-runtime")]
	TryRuntime(try_runtime_cli::TryRuntimeCmd),

	/// Try some command against runtime state. Note: `try-runtime` feature must be enabled.
	#[cfg(not(feature = "try-runtime"))]
	TryRuntime,

	/// Key management cli utilities
	#[command(subcommand)]
	Key(sc_cli::KeySubcommand),

	/// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
	Verify(sc_cli::VerifyCmd),

	/// Generate a seed that provides a vanity address.
	Vanity(sc_cli::VanityCmd),

	/// Sign a message, with a given (secret) key.
	Sign(sc_cli::SignCmd),

	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Db meta columns information.
	ChainInfo(sc_cli::ChainInfoCmd),
}

impl Cli {
	/// Partially clones the Cli options, excluding the subcommand.
	/// WARNING: Use with caution and only if you're certain about its effects.
	pub fn partial_clone(&self) -> Cli {
		Cli {
			subcommand: None,
			run: self.run.clone(),
			no_hardware_benchmarks: self.no_hardware_benchmarks,
			unsafe_da_sync: self.unsafe_da_sync,
			storage_monitor: self.storage_monitor.clone(),
			kate_rpc_enabled: self.kate_rpc_enabled,
			kate_rpc_metrics_enabled: self.kate_rpc_metrics_enabled,
			kate_max_cells_size: self.kate_max_cells_size,
			#[cfg(feature = "grandpa-justifications")]
			grandpa_justification_period: self.grandpa_justification_period,
		}
	}
}
