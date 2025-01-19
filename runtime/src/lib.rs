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

//! The Data Availability runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 512.
#![recursion_limit = "512"]
#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]

pub(crate) const LOG_TARGET: &str = "da-runtime";

pub mod apis;
pub mod constants;
#[cfg(test)]
mod header_extension_builder_data_tests;
pub mod impls;
#[cfg(test)]
mod impls_tests;
pub mod kate;
mod primitives;
pub mod transaction_filter;
mod version;
mod weights;

pub use avail_core::currency::{Balance, AVAIL, CENTS, MILLICENTS};
pub use avail_core::{header::Header as DaHeader, AppId};
use constants::time::*;
pub use frame_support::{
	construct_runtime, debug,
	dispatch::DispatchClass,
	pallet_prelude::Get,
	parameter_types,
	traits::{
		ConstU32, ContainsLengthBound, Currency, EitherOfDiverse, EqualPrivilegeOnly, Everything,
		ExtrinsicCall, Imbalance, KeyOwnerProofSystem, OnUnbalanced, Randomness, SortedMembers,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		ConstantMultiplier, IdentityFee, Weight,
	},
	PalletId, StorageValue,
};
pub use impls::BlockHashCount;
pub use pallet_balances::Call as BalancesCall;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
#[cfg(feature = "std")]
pub use pallet_staking::StakerStatus;
pub use primitives::*;
use sp_core::OpaqueMetadata;

pub use sp_runtime::{Perbill, Percent, Permill, RuntimeDebug};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
pub use version::VERSION;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. This means the client is built with \
		 `SKIP_WASM_BUILD` flag and it is only usable for production chains. Please rebuild with \
		 the flag disabled.",
	)
}

pub mod voter_bags;

/// The version information used to identify this runtime when compiled natively.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime
	{
		System: frame_system = 0,
		Utility: pallet_utility = 1,
		Babe: pallet_babe = 2,
		Timestamp: pallet_timestamp = 3,
		Authorship: pallet_authorship = 4,
		Indices: pallet_indices = 5,
		Balances: pallet_balances = 6,
		TransactionPayment: pallet_transaction_payment = 7,

		ElectionProviderMultiPhase: pallet_election_provider_multi_phase = 9,
		Staking: pallet_staking = 10,
		Session: pallet_session = 11,
		// Democracy: pallet_democracy = 12,
		// Council: pallet_collective::<Instance1> = 13,
		TechnicalCommittee: pallet_collective::<Instance2> = 14,
		// Elections: pallet_elections_phragmen = 15,
		// TechnicalMembership: pallet_membership::<Instance1> = 16,
		Grandpa: pallet_grandpa = 17,
		Treasury: pallet_treasury = 18,

		Sudo: pallet_sudo = 19,
		ImOnline: pallet_im_online = 20,
		AuthorityDiscovery: pallet_authority_discovery = 21,
		Offences: pallet_offences = 22,
		Historical: pallet_session_historical = 23,

		Scheduler: pallet_scheduler = 24,
		Mmr: pallet_mmr = 27,
		// BagsList: pallet_bags_list = 28,

		// DA module
		DataAvailability: da_control = 29,

		// Old: Nomad - don't use id 30,31,32

		// More from upgrade to v0.9.33
		Preimage: pallet_preimage = 33,
		Multisig: pallet_multisig = 34,
		VoterList: pallet_bags_list::<Instance1> = 35,
		NominationPools: pallet_nomination_pools = 36,
		Identity: pallet_identity = 37,
		Mandate: pallet_mandate = 38,
		Vector: pallet_vector = 39,
		Proxy: pallet_proxy = 40,
		TxPause: pallet_tx_pause = 41,
		TreasuryCommittee: pallet_collective::<Instance1> = 42,
		Fusion: pallet_fusion = 43,
	}
);

/// MMR helper types.
pub(crate) mod mmr {
	pub use pallet_mmr::primitives::*;

	use super::Runtime;

	pub type Leaf = <<Runtime as pallet_mmr::Config>::LeafData as LeafDataProvider>::LeafData;
	pub type Hash = <Hashing as sp_runtime::traits::Hash>::Output;
	pub type Hashing = <Runtime as pallet_mmr::Config>::Hashing;
}

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[pallet_utility, crate::Utility]
		[pallet_babe, crate::Babe]
		[pallet_timestamp, crate::Timestamp]
		[pallet_indices, crate::Indices]
		[pallet_balances, crate::Balances]
		[pallet_election_provider_multi_phase, crate::ElectionProviderMultiPhase]
		[pallet_staking, crate::Staking]
		[pallet_collective, crate::TechnicalCommittee]
		[pallet_grandpa, crate::Grandpa]
		[pallet_treasury, crate::Treasury]
		[pallet_im_online, crate::ImOnline]
		[pallet_scheduler, crate::Scheduler]
		[pallet_mmr, crate::Mmr]

		[frame_system, SystemBench::<Runtime>]
		[da_control, crate::DataAvailability]
		[pallet_identity, crate::Identity]
		[pallet_mandate, crate::Mandate]
		[pallet_vector, crate::Vector]
		[pallet_proxy, crate::Proxy]
		[pallet_tx_pause, crate::TxPause]
		[pallet_collective, crate::TreasuryCommittee]
		[pallet_fusion, crate::Fusion]
	);
}

#[cfg(test)]
mod tests {
	use core::mem::size_of;
	use std::collections::HashSet;

	use frame_election_provider_support::NposSolution;
	use frame_support::traits::WhitelistedStorageKeys;
	use frame_system::offchain::CreateSignedTransaction;
	use hex_literal::hex;
	use sp_core::hexdisplay::HexDisplay;
	use sp_keyring::AccountKeyring::Bob;
	use sp_runtime::{MultiAddress, UpperOf};
	use test_case::test_case;

	use super::*;

	/// This test was used to detect any missing support of `TryState` needed for `try-runtime`
	/// feature.
	#[cfg(feature = "try-runtime")]
	#[allow(dead_code)]
	fn check_try_runtime_support_on_pallets() -> Result<(), &'static str> {
		use crate::impls::{TechnicalCollective, TreasuryCollective};
		use frame_support::traits::{TryState, TryStateSelect::All};
		use sp_runtime::traits::Zero;

		let block = Zero::zero();

		<frame_system::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;

		<pallet_utility::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_babe::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_timestamp::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_authorship::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_indices::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_balances::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_transaction_payment::Pallet<Runtime> as TryState<BlockNumber>>::try_state(
			block, All,
		)?;
		<pallet_election_provider_multi_phase::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_staking::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_session::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_collective::Pallet<Runtime, TechnicalCollective> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_grandpa::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_treasury::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_sudo::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_im_online::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_authority_discovery::Pallet<Runtime> as TryState<BlockNumber>>::try_state(
			block, All,
		)?;
		<pallet_offences::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_session::historical::Pallet<Runtime> as TryState<BlockNumber>>::try_state(
			block, All,
		)?;
		<pallet_scheduler::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_mmr::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<da_control::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_preimage::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_multisig::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_bags_list::Pallet<Runtime, pallet_bags_list::Instance1> as TryState<
			BlockNumber,
		>>::try_state(block, All)?;
		<pallet_identity::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_mandate::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_vector::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_nomination_pools::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_proxy::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_collective::Pallet<Runtime, TreasuryCollective> as TryState<BlockNumber>>::try_state(block, All)?;
		<pallet_fusion::Pallet<Runtime> as TryState<BlockNumber>>::try_state(block, All)?;
		Ok(())
	}

	#[test]
	fn check_whitelist() {
		let whitelist: HashSet<String> = AllPalletsWithSystem::whitelisted_storage_keys()
			.iter()
			.map(|e| HexDisplay::from(&e.key).to_string())
			.collect();

		// Block Number
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac")
		);
		// Total Issuance
		assert!(
			whitelist.contains("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80")
		);
		// Execution Phase
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a")
		);
		// Event Count
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850")
		);
		// System Events
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7")
		);
		// System BlockWeight
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef734abf5cb34d6244378cddbf18e849d96")
		);
	}

	#[test]
	fn validate_transaction_submitter_bounds() {
		fn is_submit_signed_transaction<T>()
		where
			T: CreateSignedTransaction<RuntimeCall>,
		{
		}

		is_submit_signed_transaction::<Runtime>();
	}

	#[test]
	fn perbill_as_onchain_accuracy() {
		type OnChainAccuracy =
			<<Runtime as pallet_election_provider_multi_phase::MinerConfig>::Solution as NposSolution>::Accuracy;
		let maximum_chain_accuracy: Vec<UpperOf<OnChainAccuracy>> = (0
			..constants::staking::MaxNominations::get())
			.map(|_| <UpperOf<OnChainAccuracy>>::from(OnChainAccuracy::one().deconstruct()))
			.collect();
		let _: UpperOf<OnChainAccuracy> = maximum_chain_accuracy
			.iter()
			.fold(0, |acc, x| acc.checked_add(*x).unwrap());
	}

	const RUNTIME_CALL_SIZE: usize = size_of::<RuntimeCall>();
	const DA_CALL_SIZE: usize = size_of::<da_control::Call<Runtime>>();
	const SYSTEM_CALL_SIZE: usize = size_of::<frame_system::Call<Runtime>>();

	#[test_case(RUNTIME_CALL_SIZE => 208)]
	#[test_case(DA_CALL_SIZE => 64)]
	#[test_case(SYSTEM_CALL_SIZE => 40)]
	fn call_size(size: usize) -> usize {
		const MAX_CALL_SIZE: usize = 208;
		assert!(
			size <= MAX_CALL_SIZE,
			"size of RuntimeCall {} is more than 208 bytes: some calls have too big arguments, use Box to reduce the
			size of RuntimeCall.
			If the limit is too strong, maybe consider increase the limit to 300.",
			size,
		);
		size
	}

	const TRANSFER_RAW : &[u8]= &hex!("b4040600008eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a4813000064a7b3b6e00d");
	/// Creates a transfer tx of 1 AVAIL to Bob.
	fn transfer_expected() -> RuntimeCall {
		RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
			dest: MultiAddress::Id(Bob.to_account_id()),
			value: AVAIL,
		})
	}

	const SET_TIMESTAMP_RAW: &[u8] = &hex!("280403000ba302ac318301");
	// `set_timestamp` extrinsic from block 13852 on DevNet.
	fn set_timestamp_expected() -> RuntimeCall {
		RuntimeCall::Timestamp(pallet_timestamp::Call::set {
			now: 1_662_985_700_003,
		})
	}

	#[test_case( &TRANSFER_RAW => transfer_expected(); "Transfer 1 AVAIL to Bob")]
	#[test_case( &SET_TIMESTAMP_RAW => set_timestamp_expected(); "set_timestamp_block_242")]
	fn decode_app_unchecked_extrinsics(mut raw_ext: &[u8]) -> RuntimeCall {
		use codec::Decode;
		let app_ext = UncheckedExtrinsic::decode(&mut raw_ext).expect("Valid raw extrinsic .qed");
		app_ext.function
	}
}
