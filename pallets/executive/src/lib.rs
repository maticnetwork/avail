// This file is part of Substrate.

// Copyright (C) 2017-2022 Parity Technologies (UK) Ltd.
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

//! # Executive Module
//!
//! The Executive module acts as the orchestration layer for the runtime. It dispatches incoming
//! extrinsic calls to the respective modules in the runtime.
//!
//! ## Overview
//!
//! The executive module is not a typical pallet providing functionality around a specific feature.
//! It is a cross-cutting framework component for the FRAME. It works in conjunction with the
//! [FRAME System module](../frame_system/index.html) to perform these cross-cutting functions.
//!
//! The Executive module provides functions to:
//!
//! - Check transaction validity.
//! - Initialize a block.
//! - Apply extrinsics.
//! - Execute a block.
//! - Finalize a block.
//! - Start an off-chain worker.
//!
//! ### Implementations
//!
//! The Executive module provides the following implementations:
//!
//! - `ExecuteBlock`: Trait that can be used to execute a block.
//! - `Executive`: Type that can be used to make the FRAME available from the runtime.
//!
//! ## Usage
//!
//! The default Substrate node template declares the [`Executive`](./struct.Executive.html) type in
//! its library.
//!
//! ### Example
//!
//! `Executive` type declaration from the node template.
//!
//! ```
//! # use sp_runtime::generic;
//! # use frame_executive as executive;
//! # pub struct UncheckedExtrinsic {};
//! # pub struct Header {};
//! # type Context = frame_system::ChainContext<Runtime>;
//! # pub type Block = generic::Block<Header, UncheckedExtrinsic>;
//! # pub type Balances = u64;
//! # pub type AllPalletsWithSystem = u64;
//! # pub enum Runtime {};
//! # use sp_runtime::transaction_validity::{
//! #    TransactionValidity, UnknownTransaction, TransactionSource,
//! # };
//! # use sp_runtime::traits::ValidateUnsigned;
//! # impl ValidateUnsigned for Runtime {
//! #     type Call = ();
//! #
//! #     fn validate_unsigned(_source: TransactionSource, _call: &Self::Call) -> TransactionValidity {
//! #         UnknownTransaction::NoUnsignedValidator.into()
//! #     }
//! # }
//! /// Executive: handles dispatch to the various modules.
//! pub type Executive = executive::Executive<Runtime, Block, Context, Runtime, AllPalletsWithSystem>;
//! ```
//!
//! ### Custom `OnRuntimeUpgrade` logic
//!
//! You can add custom logic that should be called in your runtime on a runtime upgrade. This is
//! done by setting an optional generic parameter. The custom logic will be called before
//! the on runtime upgrade logic of all modules is called.
//!
//! ```
//! # use sp_runtime::generic;
//! # use frame_executive as executive;
//! # pub struct UncheckedExtrinsic {};
//! # pub struct Header {};
//! # type Context = frame_system::ChainContext<Runtime>;
//! # pub type Block = generic::Block<Header, UncheckedExtrinsic>;
//! # pub type Balances = u64;
//! # pub type AllPalletsWithSystem = u64;
//! # pub enum Runtime {};
//! # use sp_runtime::transaction_validity::{
//! #    TransactionValidity, UnknownTransaction, TransactionSource,
//! # };
//! # use sp_runtime::traits::ValidateUnsigned;
//! # impl ValidateUnsigned for Runtime {
//! #     type Call = ();
//! #
//! #     fn validate_unsigned(_source: TransactionSource, _call: &Self::Call) -> TransactionValidity {
//! #         UnknownTransaction::NoUnsignedValidator.into()
//! #     }
//! # }
//! struct CustomOnRuntimeUpgrade;
//! impl frame_support::traits::OnRuntimeUpgrade for CustomOnRuntimeUpgrade {
//!     fn on_runtime_upgrade() -> frame_support::weights::Weight {
//!         // Do whatever you want.
//!         frame_support::weights::Weight::zero()
//!     }
//! }
//!
//! pub type Executive = executive::Executive<Runtime, Block, Context, Runtime, AllPalletsWithSystem, CustomOnRuntimeUpgrade>;
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Encode};
#[cfg(feature = "try-runtime")]
use da_primitives::asdr::AppId;
use da_primitives::asdr::GetAppId;
use frame_support::{
	dispatch::{DispatchClass, DispatchInfo, GetDispatchInfo, PostDispatchInfo},
	pallet_prelude::InvalidTransaction,
	traits::{
		EnsureInherentsAreFirst, ExecuteBlock, OffchainWorker, OnFinalize, OnIdle, OnInitialize,
		OnRuntimeUpgrade,
	},
	weights::Weight,
};
use sp_runtime::{
	generic::Digest,
	traits::{
		self, Applyable, CheckEqual, Checkable, Dispatchable, Header, NumberFor, One,
		ValidateUnsigned, Zero,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,
};
use sp_std::{marker::PhantomData, vec::Vec};

pub type CheckedOf<E, C> = <E as Checkable<C>>::Checked;
pub type CallOf<E, C> = <CheckedOf<E, C> as Applyable>::Call;
pub type OriginOf<E, C> = <CallOf<E, C> as Dispatchable>::RuntimeOrigin;

/// Main entry point for certain runtime actions as e.g. `execute_block`.
///
/// Generic parameters:
/// - `System`: Something that implements `frame_system::Config`
/// - `Block`: The block type of the runtime
/// - `Context`: The context that is used when checking an extrinsic.
/// - `UnsignedValidator`: The unsigned transaction validator of the runtime.
/// - `AllPalletsWithSystem`: Tuple that contains all pallets including frame system pallet. Will be
///   used to call hooks e.g. `on_initialize`.
/// - `OnRuntimeUpgrade`: Custom logic that should be called after a runtime upgrade. Modules are
///   already called by `AllPalletsWithSystem`. It will be called before all modules will be called.
pub struct Executive<
	System,
	Block,
	Context,
	UnsignedValidator,
	AllPalletsWithSystem,
	OnRuntimeUpgrade = (),
>(
	PhantomData<(
		System,
		Block,
		Context,
		UnsignedValidator,
		AllPalletsWithSystem,
		OnRuntimeUpgrade,
	)>,
);

impl<
		System: frame_system::Config + EnsureInherentsAreFirst<Block>,
		Block: traits::Block<Header = System::Header, Hash = System::Hash>,
		Context: Default,
		UnsignedValidator,
		AllPalletsWithSystem: OnRuntimeUpgrade
			+ OnInitialize<System::BlockNumber>
			+ OnIdle<System::BlockNumber>
			+ OnFinalize<System::BlockNumber>
			+ OffchainWorker<System::BlockNumber>,
		COnRuntimeUpgrade: OnRuntimeUpgrade,
	> ExecuteBlock<Block>
	for Executive<System, Block, Context, UnsignedValidator, AllPalletsWithSystem, COnRuntimeUpgrade>
where
	Block::Extrinsic: Checkable<Context> + Codec + GetAppId,
	CheckedOf<Block::Extrinsic, Context>: Applyable + GetDispatchInfo,
	CallOf<Block::Extrinsic, Context>:
		Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	OriginOf<Block::Extrinsic, Context>: From<Option<System::AccountId>>,
	UnsignedValidator: ValidateUnsigned<Call = CallOf<Block::Extrinsic, Context>>,
{
	fn execute_block(block: Block) {
		Executive::<
			System,
			Block,
			Context,
			UnsignedValidator,
			AllPalletsWithSystem,
			COnRuntimeUpgrade,
		>::execute_block(block);
	}
}

#[cfg(feature = "try-runtime")]
impl<
		System: frame_system::Config + EnsureInherentsAreFirst<Block>,
		Block: traits::Block<Header = System::Header, Hash = System::Hash>,
		Context: Default,
		UnsignedValidator,
		AllPalletsWithSystem: OnRuntimeUpgrade
			+ OnInitialize<System::BlockNumber>
			+ OnIdle<System::BlockNumber>
			+ OnFinalize<System::BlockNumber>
			+ OffchainWorker<System::BlockNumber>
			+ frame_support::traits::TryState<System::BlockNumber>,
		COnRuntimeUpgrade: OnRuntimeUpgrade,
	> Executive<System, Block, Context, UnsignedValidator, AllPalletsWithSystem, COnRuntimeUpgrade>
where
	Block::Extrinsic: Checkable<Context> + Codec + GetAppId,
	CheckedOf<Block::Extrinsic, Context>: Applyable + GetDispatchInfo,
	CallOf<Block::Extrinsic, Context>:
		Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	OriginOf<Block::Extrinsic, Context>: From<Option<System::AccountId>>,
	UnsignedValidator: ValidateUnsigned<Call = CallOf<Block::Extrinsic, Context>>,
{
	/// Execute given block, but don't as strict is the normal block execution.
	///
	/// Some checks can be disabled via:
	///
	/// - `state_root_check`
	/// - `signature_check`
	///
	/// Should only be used for testing ONLY.
	pub fn try_execute_block(
		block: Block,
		state_root_check: bool,
		signature_check: bool,
		select: frame_try_runtime::TryStateSelect,
	) -> Result<Weight, &'static str> {
		frame_support::log::info!(
			target: "frame::executive",
			"try-runtime: executing block #{:?} / state root check: {:?} / signature check: {:?} / try-state-select: {:?}",
			block.header().number(),
			state_root_check,
			signature_check,
			select,
		);

		Self::initialize_block(block.header());
		Self::initial_checks(&block);

		let (header, extrinsics) = block.deconstruct();

		let try_apply_extrinsic = |uxt: Block::Extrinsic| -> ApplyExtrinsicResult {
			sp_io::init_tracing();
			let encoded = uxt.encode();
			let encoded_len = encoded.len();

			// skip signature verification.
			let xt = if signature_check {
				uxt.check(&Default::default())
			} else {
				uxt.unchecked_into_checked_i_know_what_i_am_doing(&Default::default())
			}?;
			<frame_system::Pallet<System>>::note_extrinsic(AppId::default(), encoded);

			let dispatch_info = xt.get_dispatch_info();
			let r = Applyable::apply::<UnsignedValidator>(xt, &dispatch_info, encoded_len)?;

			<frame_system::Pallet<System>>::note_applied_extrinsic(&r, dispatch_info);

			Ok(r.map(|_| ()).map_err(|e| e.error))
		};

		for e in extrinsics {
			if let Err(err) = try_apply_extrinsic(e.clone()) {
				frame_support::log::error!(
					target: "runtime::executive", "executing transaction {:?} failed due to {:?}. Aborting the rest of the block execution.",
					e,
					err,
				);
				break;
			}
		}

		// post-extrinsics book-keeping
		<frame_system::Pallet<System>>::note_finished_extrinsics();
		Self::idle_and_finalize_hook(*header.number());

		// run the try-state checks of all pallets, ensuring they don't alter any state.
		let _guard = frame_support::StorageNoopGuard::default();
		<AllPalletsWithSystem as frame_support::traits::TryState<System::BlockNumber>>::try_state(
			*header.number(),
			select,
		)
		.map_err(|e| {
			frame_support::log::error!(target: "runtime::executive", "failure: {:?}", e);
			e
		})?;
		drop(_guard);

		// do some of the checks that would normally happen in `final_checks`, but perhaps skip
		// the state root check.
		{
			let new_header = <frame_system::Pallet<System>>::finalize();
			let items_zip = header
				.digest()
				.logs()
				.iter()
				.zip(new_header.digest().logs().iter());
			for (header_item, computed_item) in items_zip {
				header_item.check_equal(computed_item);
				assert!(
					header_item == computed_item,
					"Digest item must match that calculated."
				);
			}

			if state_root_check {
				let storage_root = new_header.state_root();
				header.state_root().check_equal(storage_root);
				assert!(
					header.state_root() == storage_root,
					"Storage root must match that calculated."
				);
			}

			assert!(
				header.extrinsics_root() == new_header.extrinsics_root(),
				"Transaction trie root must be valid.",
			);
		}

		Ok(frame_system::Pallet::<System>::block_weight().total())
	}

	/// Execute all `OnRuntimeUpgrade` of this runtime, including the pre and post migration checks.
	///
	/// Runs the try-state code both before and after the migration function if `checks` is set to
	/// `true`. Also, if set to `true`, it runs the `pre_upgrade` and `post_upgrade` hooks.
	pub fn try_runtime_upgrade(
		checks: frame_try_runtime::UpgradeCheckSelect,
	) -> Result<Weight, &'static str> {
		if checks.try_state() {
			let _guard = frame_support::StorageNoopGuard::default();
			<AllPalletsWithSystem as frame_support::traits::TryState<System::BlockNumber>>::try_state(
				frame_system::Pallet::<System>::block_number(),
				frame_try_runtime::TryStateSelect::All,
			)?;
		}

		let weight =
			<(COnRuntimeUpgrade, AllPalletsWithSystem) as OnRuntimeUpgrade>::try_on_runtime_upgrade(
				checks.pre_and_post(),
			)?;

		if checks.try_state() {
			let _guard = frame_support::StorageNoopGuard::default();
			<AllPalletsWithSystem as frame_support::traits::TryState<System::BlockNumber>>::try_state(
				frame_system::Pallet::<System>::block_number(),
				frame_try_runtime::TryStateSelect::All,
			)?;
		}

		Ok(weight)
	}
}

impl<
		System: frame_system::Config + EnsureInherentsAreFirst<Block>,
		Block: traits::Block<Header = System::Header, Hash = System::Hash>,
		Context: Default,
		UnsignedValidator,
		AllPalletsWithSystem: OnRuntimeUpgrade
			+ OnInitialize<System::BlockNumber>
			+ OnIdle<System::BlockNumber>
			+ OnFinalize<System::BlockNumber>
			+ OffchainWorker<System::BlockNumber>,
		COnRuntimeUpgrade: OnRuntimeUpgrade,
	> Executive<System, Block, Context, UnsignedValidator, AllPalletsWithSystem, COnRuntimeUpgrade>
where
	Block::Extrinsic: Checkable<Context> + Codec + GetAppId,
	CheckedOf<Block::Extrinsic, Context>: Applyable + GetDispatchInfo,
	CallOf<Block::Extrinsic, Context>:
		Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	OriginOf<Block::Extrinsic, Context>: From<Option<System::AccountId>>,
	UnsignedValidator: ValidateUnsigned<Call = CallOf<Block::Extrinsic, Context>>,
{
	/// Execute all `OnRuntimeUpgrade` of this runtime, and return the aggregate weight.
	pub fn execute_on_runtime_upgrade() -> frame_support::weights::Weight {
		<(COnRuntimeUpgrade, AllPalletsWithSystem) as OnRuntimeUpgrade>::on_runtime_upgrade()
	}

	/// Start the execution of a particular block.
	pub fn initialize_block(header: &System::Header) {
		sp_io::init_tracing();
		sp_tracing::enter_span!(sp_tracing::Level::TRACE, "init_block");
		let digests = Self::extract_pre_digest(header);
		Self::initialize_block_impl(header.number(), header.parent_hash(), &digests);
	}

	fn extract_pre_digest(header: &System::Header) -> Digest {
		let mut digest = <Digest>::default();
		header.digest().logs().iter().for_each(|d| {
			if d.as_pre_runtime().is_some() {
				digest.push(d.clone())
			}
		});
		digest
	}

	fn initialize_block_impl(
		block_number: &System::BlockNumber,
		parent_hash: &System::Hash,
		digest: &Digest,
	) {
		// Reset events before apply runtime upgrade hook.
		// This is required to preserve events from runtime upgrade hook.
		// This means the format of all the event related storages must always be compatible.
		<frame_system::Pallet<System>>::reset_events();

		let mut weight = Weight::zero();
		if Self::runtime_upgraded() {
			weight = weight.saturating_add(Self::execute_on_runtime_upgrade());
		}
		<frame_system::Pallet<System>>::initialize(block_number, parent_hash, digest);
		weight = weight.saturating_add(<AllPalletsWithSystem as OnInitialize<
			System::BlockNumber,
		>>::on_initialize(*block_number));
		weight = weight.saturating_add(
			<System::BlockWeights as frame_support::traits::Get<_>>::get().base_block,
		);
		<frame_system::Pallet<System>>::register_extra_weight_unchecked(
			weight,
			DispatchClass::Mandatory,
		);

		frame_system::Pallet::<System>::note_finished_initialize();
	}

	/// Returns if the runtime was upgraded since the last time this function was called.
	fn runtime_upgraded() -> bool {
		let last = frame_system::LastRuntimeUpgrade::<System>::get();
		let current = <System::Version as frame_support::traits::Get<_>>::get();

		if last.map(|v| v.was_upgraded(&current)).unwrap_or(true) {
			frame_system::LastRuntimeUpgrade::<System>::put(
				frame_system::LastRuntimeUpgradeInfo::from(current),
			);
			true
		} else {
			false
		}
	}

	fn initial_checks(block: &Block) {
		sp_tracing::enter_span!(sp_tracing::Level::TRACE, "initial_checks");
		let header = block.header();

		// Check that `parent_hash` is correct.
		let n = *header.number();
		assert!(
			n > System::BlockNumber::zero()
				&& <frame_system::Pallet<System>>::block_hash(n - System::BlockNumber::one())
					== *header.parent_hash(),
			"Parent hash should be valid.",
		);

		if let Err(i) = System::ensure_inherents_are_first(block) {
			panic!("Invalid inherent position for extrinsic at index {}", i);
		}
	}

	/// Actually execute all transitions for `block`.
	pub fn execute_block(block: Block) {
		sp_io::init_tracing();
		sp_tracing::within_span! {
			sp_tracing::info_span!("execute_block", ?block);

			Self::initialize_block(block.header());

			// any initial checks
			Self::initial_checks(&block);

			let signature_batching = sp_runtime::SignatureBatching::start();

			// execute extrinsics
			let (header, extrinsics) = block.deconstruct();
			Self::execute_extrinsics_with_book_keeping(extrinsics, *header.number());

			if !signature_batching.verify() {
				panic!("Signature verification failed.");
			}

			// any final checks
			Self::final_checks(&header);
		}
	}

	/// Execute given extrinsics and take care of post-extrinsics book-keeping.
	fn execute_extrinsics_with_book_keeping(
		extrinsics: Vec<Block::Extrinsic>,
		block_number: NumberFor<Block>,
	) {
		extrinsics.into_iter().for_each(|e| {
			if let Err(e) = Self::apply_extrinsic(e) {
				let err: &'static str = e.into();
				panic!("{}", err)
			}
		});

		// post-extrinsics book-keeping
		<frame_system::Pallet<System>>::note_finished_extrinsics();

		Self::idle_and_finalize_hook(block_number);
	}

	/// Finalize the block - it is up the caller to ensure that all header fields are valid
	/// except state-root.
	pub fn finalize_block() -> System::Header {
		sp_io::init_tracing();
		sp_tracing::enter_span!(sp_tracing::Level::TRACE, "finalize_block");
		<frame_system::Pallet<System>>::note_finished_extrinsics();
		let block_number = <frame_system::Pallet<System>>::block_number();

		Self::idle_and_finalize_hook(block_number);

		<frame_system::Pallet<System>>::finalize()
	}

	fn idle_and_finalize_hook(block_number: NumberFor<Block>) {
		let weight = <frame_system::Pallet<System>>::block_weight();
		let max_weight = <System::BlockWeights as frame_support::traits::Get<_>>::get().max_block;
		let remaining_weight = max_weight.saturating_sub(weight.total());

		if remaining_weight.all_gt(Weight::zero()) {
			let used_weight = <AllPalletsWithSystem as OnIdle<System::BlockNumber>>::on_idle(
				block_number,
				remaining_weight,
			);
			<frame_system::Pallet<System>>::register_extra_weight_unchecked(
				used_weight,
				DispatchClass::Mandatory,
			);
		}

		<AllPalletsWithSystem as OnFinalize<System::BlockNumber>>::on_finalize(block_number);
	}

	/// Apply extrinsic outside of the block execution function.
	///
	/// This doesn't attempt to validate anything regarding the block, but it builds a list of uxt
	/// hashes.
	pub fn apply_extrinsic(uxt: Block::Extrinsic) -> ApplyExtrinsicResult {
		sp_io::init_tracing();
		let encoded = uxt.encode();
		let encoded_len = encoded.len();
		Self::apply_extrinsic_with_len(uxt, encoded_len, encoded)
	}

	/// Actually apply an extrinsic given its `encoded_len`; this doesn't note its hash.
	fn apply_extrinsic_with_len(
		uxt: Block::Extrinsic,
		encoded_len: usize,
		to_note: Vec<u8>,
	) -> ApplyExtrinsicResult {
		let app_id = uxt.app_id();
		sp_tracing::enter_span!(sp_tracing::info_span!("apply_extrinsic",
				ext=?sp_core::hexdisplay::HexDisplay::from(&uxt.encode())));
		// Verify that the signature is good.
		let xt = uxt.check(&Default::default())?;

		// We don't need to make sure to `note_extrinsic` only after we know it's going to be
		// executed to prevent it from leaking in storage since at this point, it will either
		// execute or panic (and revert storage changes).
		<frame_system::Pallet<System>>::note_extrinsic(app_id, to_note);

		// AUDIT: Under no circumstances may this function panic from here onwards.

		// Decode parameters and dispatch
		let dispatch_info = xt.get_dispatch_info();
		let r = Applyable::apply::<UnsignedValidator>(xt, &dispatch_info, encoded_len)?;

		// Mandatory(inherents) are not allowed to fail.
		//
		// The entire block should be discarded if an inherent fails to apply. Otherwise
		// it may open an attack vector.
		if r.is_err() && dispatch_info.class == DispatchClass::Mandatory {
			return Err(InvalidTransaction::BadMandatory.into());
		}

		<frame_system::Pallet<System>>::note_applied_extrinsic(&r, dispatch_info);

		Ok(r.map(|_| ()).map_err(|e| e.error))
	}

	fn final_checks(header: &System::Header) {
		sp_tracing::enter_span!(sp_tracing::Level::TRACE, "final_checks");
		// remove temporaries
		let new_header = <frame_system::Pallet<System>>::finalize();

		// check digest
		assert_eq!(
			header.digest().logs().len(),
			new_header.digest().logs().len(),
			"Number of digest items must match that calculated."
		);
		let items_zip = header
			.digest()
			.logs()
			.iter()
			.zip(new_header.digest().logs().iter());
		for (header_item, computed_item) in items_zip {
			header_item.check_equal(computed_item);
			assert!(
				header_item == computed_item,
				"Digest item must match that calculated."
			);
		}

		// check storage root.
		let storage_root = new_header.state_root();
		header.state_root().check_equal(storage_root);
		assert!(
			header.state_root() == storage_root,
			"Storage root must match that calculated."
		);

		assert!(
			header.extrinsics_root() == new_header.extrinsics_root(),
			"Transaction trie root must be valid.",
		);
	}

	/// Check a given signed transaction for validity. This doesn't execute any
	/// side-effects; it merely checks whether the transaction would panic if it were included or
	/// not.
	///
	/// Changes made to storage should be discarded.
	pub fn validate_transaction(
		source: TransactionSource,
		uxt: Block::Extrinsic,
		block_hash: Block::Hash,
	) -> TransactionValidity {
		sp_io::init_tracing();
		use sp_tracing::{enter_span, within_span};

		<frame_system::Pallet<System>>::initialize(
			&(frame_system::Pallet::<System>::block_number() + One::one()),
			&block_hash,
			&Default::default(),
		);

		enter_span! { sp_tracing::Level::TRACE, "validate_transaction" };

		let encoded_len = within_span! { sp_tracing::Level::TRACE, "using_encoded";
			uxt.using_encoded(|d| d.len())
		};

		let xt = within_span! { sp_tracing::Level::TRACE, "check";
			uxt.check(&Default::default())
		}?;

		let dispatch_info = within_span! { sp_tracing::Level::TRACE, "dispatch_info";
			xt.get_dispatch_info()
		};

		if dispatch_info.class == DispatchClass::Mandatory {
			return Err(InvalidTransaction::MandatoryValidation.into());
		}

		within_span! {
			sp_tracing::Level::TRACE, "validate";
			xt.validate::<UnsignedValidator>(source, &dispatch_info, encoded_len)
		}
	}

	/// Start an offchain worker and generate extrinsics.
	pub fn offchain_worker(header: &System::Header) {
		sp_io::init_tracing();
		// We need to keep events available for offchain workers,
		// hence we initialize the block manually.
		// OffchainWorker RuntimeApi should skip initialization.
		let digests = header.digest().clone();

		<frame_system::Pallet<System>>::initialize(header.number(), header.parent_hash(), &digests);

		// Frame system only inserts the parent hash into the block hashes as normally we don't know
		// the hash for the header before. However, here we are aware of the hash and we can add it
		// as well.
		frame_system::BlockHash::<System>::insert(header.number(), header.hash());

		<AllPalletsWithSystem as OffchainWorker<System::BlockNumber>>::offchain_worker(
			*header.number(),
		)
	}
}

#[cfg(test)]
mod tests {
	use da_control::CheckAppId;
	use da_primitives::{
		asdr::DataLookup, currency::AVL, extension::v1, Header as DaHeader, HeaderExtension,
	};
	use frame_support::{
		assert_err,
		dispatch::CallableCallFor,
		traits::{Currency, LockIdentifier, LockableCurrency, WithdrawReasons},
		weights::{Weight, WeightToFee},
	};
	use frame_system::{ChainContext, LastRuntimeUpgradeInfo};
	use hex_literal::hex;
	use mocked_runtime::{
		custom::TEST_KEY, AccountId, AllPalletsWithSystem, Balance, Balances, Block as TestBlock,
		BlockNumber, Custom, Runtime, RuntimeCall, RuntimeVersionTestValues, SignedExtra, System,
		TestXt, CUSTOM_ON_RUNTIME_KEY,
	};
	use sp_core::H256;
	use sp_runtime::{
		generic::{Block, DigestItem, Era},
		testing::Digest,
		traits::{BlakeTwo256, Block as BlockT},
		transaction_validity::{
			InvalidTransaction, TransactionValidityError, UnknownTransaction, ValidTransaction,
		},
		DispatchError,
	};

	use super::*;

	type SystemCall = CallableCallFor<System, Runtime>;
	type BalancesCall = CallableCallFor<Balances, Runtime>;
	type CustomCall = CallableCallFor<Custom, Runtime>;

	struct CustomOnRuntimeUpgrade;
	impl OnRuntimeUpgrade for CustomOnRuntimeUpgrade {
		fn on_runtime_upgrade() -> Weight {
			sp_io::storage::set(TEST_KEY, "custom_upgrade".as_bytes());
			sp_io::storage::set(CUSTOM_ON_RUNTIME_KEY, &true.encode());
			System::deposit_event(frame_system::Event::CodeUpdated);
			Weight::from_ref_time(100)
		}
	}

	type Executive = super::Executive<
		Runtime,
		TestBlock,
		ChainContext<Runtime>,
		Runtime,
		AllPalletsWithSystem,
		CustomOnRuntimeUpgrade,
	>;

	fn extra(nonce: u64, fee: Balance) -> SignedExtra {
		(
			frame_system::CheckEra::from(Era::Immortal),
			frame_system::CheckNonce::from(nonce),
			frame_system::CheckWeight::new(),
			pallet_transaction_payment::ChargeTransactionPayment::from(fee),
			CheckAppId::default(),
		)
	}

	fn sign_extra(who: u64, nonce: u64, fee: Balance) -> Option<(u64, SignedExtra)> {
		Some((who, extra(nonce, fee)))
	}

	fn call_transfer(dest: u64, value: Balance) -> RuntimeCall {
		RuntimeCall::Balances(BalancesCall::transfer { dest, value })
	}

	// NOTE: All these constants are double-checked by `constants_cost` test.
	/// Cost of block initialization, mostly `<AllPalletsWithSystem>::on_initialize(...)`
	const BLOCK_INIT_COST: Weight = Weight::from_ref_time(175);
	/// Cost of the `on_idle` section before the block finalization, mostly
	/// `<AllPalletsWithSystem>::on_idle(...)`.
	const BLOCK_IDLE_COST: Weight = Weight::from_ref_time(175);
	/// Base cost of the block initialization.
	const BLOCK_BASE_COST: Weight = Weight::from_ref_time(358_523_000);
	/// Base cost per extrinsic.
	const EXTRINSIC_BASE_COST: Weight = Weight::from_ref_time(98_974_000);

	/// It ensures that the following constants used for testing are valid
	#[test]
	fn constant_cost() {
		new_test_ext(1).execute_with(|| {
			let block_weights = <Runtime as frame_system::Config>::BlockWeights::get();

			// Verify `BLOCK_INIT_COST`
			let mut init_weight: Weight = frame_system::Pallet::<Runtime>::on_initialize(1);
			init_weight = init_weight
				.checked_add(&<AllPalletsWithSystem as OnInitialize<u32>>::on_initialize(
					1,
				))
				.expect("All pallet block initialization must be cheap .qed");
			assert_eq!(init_weight, BLOCK_INIT_COST);

			// Verify `BLOCK_BASE_COST`
			assert_eq!(block_weights.base_block, BLOCK_BASE_COST);

			// Verify `BLOCK_IDLE_COST`
			let mut remaining_weight = block_weights
				.max_block
				.saturating_sub(BLOCK_INIT_COST + BLOCK_BASE_COST);
			assert_ne!(remaining_weight, Zero::zero());

			let mut idle_weight = <frame_system::Pallet<Runtime> as OnIdle<BlockNumber>>::on_idle(
				1,
				remaining_weight,
			);
			remaining_weight = remaining_weight.saturating_sub(idle_weight);
			idle_weight =
				<AllPalletsWithSystem as OnIdle<BlockNumber>>::on_idle(1, remaining_weight)
					.saturating_add(idle_weight);

			assert_eq!(idle_weight, BLOCK_IDLE_COST);
			assert_ne!(remaining_weight, Zero::zero());

			// Verify `EXTRINSIC_BASE_COST`.
			assert_eq!(
				block_weights.get(DispatchClass::Normal).base_extrinsic,
				EXTRINSIC_BASE_COST
			);
		});
	}

	#[test]
	fn balance_transfer_dispatch_works() {
		const INIT_BALANCE: Balance = 211 * AVL;
		const TRANSFER_BALANCE: Balance = 69 * AVL;

		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();
		pallet_balances::GenesisConfig::<Runtime> {
			balances: vec![(1, INIT_BALANCE)],
		}
		.assimilate_storage(&mut t)
		.unwrap();
		let xt = TestXt::new(call_transfer(2, TRANSFER_BALANCE), sign_extra(1, 0, 0));
		let weight = xt.get_dispatch_info().weight
			+ <Runtime as frame_system::Config>::BlockWeights::get()
				.get(DispatchClass::Normal)
				.base_extrinsic;
		let fee: Balance =
			<Runtime as pallet_transaction_payment::Config>::WeightToFee::weight_to_fee(&weight);
		let mut t = sp_io::TestExternalities::new(t);
		t.execute_with(|| {
			Executive::initialize_block(&Header::new(
				1,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));
			let init_balance = <pallet_balances::Pallet<Runtime>>::total_balance(&1);
			assert_eq!(init_balance, INIT_BALANCE);
			let encoded_len: Balance = xt.encode().len() as Balance;
			let r = Executive::apply_extrinsic(xt);
			assert!(r.is_ok());
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::total_balance(&1),
				INIT_BALANCE - TRANSFER_BALANCE - fee - encoded_len
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::total_balance(&2),
				TRANSFER_BALANCE
			);
		});
	}

	fn new_test_ext(balance_factor: Balance) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();
		pallet_balances::GenesisConfig::<Runtime> {
			balances: vec![(1, 111 * AVL * balance_factor)],
		}
		.assimilate_storage(&mut t)
		.unwrap();
		t.into()
	}

	fn new_test_ext_v0(balance_factor: Balance) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();
		pallet_balances::GenesisConfig::<Runtime> {
			balances: vec![(1, 111 * balance_factor)],
		}
		.assimilate_storage(&mut t)
		.unwrap();
		(t, sp_runtime::StateVersion::V0).into()
	}

	fn make_header(state_root: H256) -> DaHeader<u32, BlakeTwo256> {
		let extension = HeaderExtension::V1(v1::HeaderExtension {
			app_lookup: DataLookup {
				size: 1,
				..Default::default()
			},
			..Default::default()
		});
		let extrinsics_root =
			hex!("03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314").into();

		DaHeader {
			parent_hash: [69u8; 32].into(),
			number: 1,
			state_root,
			extrinsics_root,
			extension,
			..Default::default()
		}
	}

	#[test]
	fn block_import_works() {
		block_import_works_inner(
			new_test_ext_v0(1),
			array_bytes::hex_n_into_unchecked(
				// "6d988b132b3dc0200c6d8102274428b9f58024fbb32c210b8b432302bc85be17",
				"f5704fe05e3ffdfeb97315b98d65e5dbbdd14a3d3232e7e673733f3bafd076b2",
			),
		);
		block_import_works_inner(
			new_test_ext(1),
			array_bytes::hex_n_into_unchecked(
				// "0b93b0592690a181d6e080c6cc73108793e0447c24ad82145c763b798d94edc7",
				"a711032732f5af9bf29e1a44ab333af1ee3cd1c774f835c27a42c9a673578f1a",
			),
		);
	}
	fn block_import_works_inner(mut ext: sp_io::TestExternalities, state_root: H256) {
		ext.execute_with(|| {
			Executive::execute_block(Block {
				header: make_header(state_root),
				extrinsics: vec![],
			});
		});
	}

	#[test]
	#[should_panic]
	fn block_import_of_bad_state_root_fails() {
		let mut header = DaHeader::default();
		header.extension = HeaderExtension::default();

		new_test_ext(1).execute_with(|| {
			Executive::execute_block(Block {
				header,
				extrinsics: vec![],
			});
		});
	}

	#[test]
	#[should_panic]
	fn block_import_of_bad_extrinsic_root_fails() {
		let mut header = DaHeader::default();
		header.state_root =
			hex!("49cd58a254ccf6abc4a023d9a22dcfc421e385527a250faec69f8ad0d8ed3e48").into();

		new_test_ext(1).execute_with(|| {
			Executive::execute_block(Block {
				header,
				extrinsics: vec![],
			});
		});
	}

	#[test]
	fn bad_extrinsic_not_inserted() {
		let mut t = new_test_ext(1);
		// bad nonce check!
		let xt = TestXt::new(call_transfer(33, 69), sign_extra(1, 30, 0));
		t.execute_with(|| {
			Executive::initialize_block(&Header::new(
				1,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));
			assert_err!(
				Executive::apply_extrinsic(xt),
				TransactionValidityError::Invalid(InvalidTransaction::Future)
			);
			assert_eq!(<frame_system::Pallet<Runtime>>::extrinsic_index(), Some(0));
		});
	}

	#[test]
	fn block_weight_limit_enforced() {
		let mut t = new_test_ext(10000);

		// on_initialize weight + base block execution weight
		let block_weights = <Runtime as frame_system::Config>::BlockWeights::get();
		let base_block_weight = Weight::from_ref_time(175) + block_weights.base_block;
		let limit = block_weights.get(DispatchClass::Normal).max_total.unwrap();

		let call = RuntimeCall::Balances(BalancesCall::transfer { dest: 33, value: 0 });
		let call_weight = TestXt::new(call.clone(), sign_extra(1, 0, 0))
			.get_dispatch_info()
			.weight;
		let extrinsic_weight = call_weight + EXTRINSIC_BASE_COST;
		let num_to_exhaust_block = limit.ref_time() / extrinsic_weight.ref_time();

		t.execute_with(|| {
			Executive::initialize_block(&Header::new(
				1,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));
			// Base block execution weight + `on_initialize` weight from the custom module.
			assert_eq!(
				<frame_system::Pallet<Runtime>>::block_weight().total(),
				base_block_weight
			);

			for nonce in 0..=num_to_exhaust_block {
				let xt = TestXt::new(call.clone(), sign_extra(1, nonce.into(), 0));

				let res = Executive::apply_extrinsic(xt);
				if nonce != num_to_exhaust_block {
					assert!(
						res.is_ok(),
						"Xt was not apply at {nonce}/{num_to_exhaust_block} iteration"
					);
					let block_weight = <frame_system::Pallet<Runtime>>::block_weight();
					let block_total_weight = block_weight.total();
					//--------------------- on_initialize + block_execution + extrinsic_base weight
					let expected_total_weight =
						(call_weight + EXTRINSIC_BASE_COST) * (nonce + 1) + base_block_weight;
					assert_eq!(block_total_weight, expected_total_weight);
					assert_eq!(
						<frame_system::Pallet<Runtime>>::extrinsic_index(),
						Some(nonce as u32 + 1)
					);
				} else {
					assert_eq!(res, Err(InvalidTransaction::ExhaustsResources.into()));
				}
			}
		});
	}

	#[test]
	fn block_weight_and_size_is_stored_per_tx() {
		let call = RuntimeCall::Balances(BalancesCall::transfer { dest: 33, value: 0 });
		let xt = TestXt::new(call.clone(), sign_extra(1, 0, 0));
		let x1 = TestXt::new(call.clone(), sign_extra(1, 1, 0));
		let x2 = TestXt::new(call, sign_extra(1, 2, 0));
		let len = xt.clone().encode().len() as u32;
		let mut t = new_test_ext(1);
		t.execute_with(|| {
			// Block execution weight + on_initialize weight from custom module
			let base_block_weight = BLOCK_IDLE_COST + BLOCK_BASE_COST;

			Executive::initialize_block(&Header::new(
				1,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));

			assert_eq!(
				<frame_system::Pallet<Runtime>>::block_weight().total(),
				base_block_weight
			);
			assert_eq!(<frame_system::Pallet<Runtime>>::all_extrinsics_len(), 0);
			let xt_weight = xt.get_dispatch_info().weight;

			assert!(Executive::apply_extrinsic(xt).unwrap().is_ok());
			assert!(Executive::apply_extrinsic(x1).unwrap().is_ok());
			assert!(Executive::apply_extrinsic(x2).unwrap().is_ok());

			// default weight for `TestXt` == encoded length.
			let extrinsic_weight = xt_weight
				+ <Runtime as frame_system::Config>::BlockWeights::get()
					.get(DispatchClass::Normal)
					.base_extrinsic;
			assert_eq!(
				<frame_system::Pallet<Runtime>>::block_weight().total(),
				base_block_weight + 3u64 * extrinsic_weight,
			);
			assert_eq!(
				<frame_system::Pallet<Runtime>>::all_extrinsics_len(),
				3 * len
			);

			let _ = <frame_system::Pallet<Runtime>>::finalize();
			// All extrinsics length cleaned on `System::finalize`
			assert_eq!(<frame_system::Pallet<Runtime>>::all_extrinsics_len(), 0);

			// New Block
			Executive::initialize_block(&Header::new(
				2,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));

			// Block weight cleaned up on `System::initialize`
			assert_eq!(
				<frame_system::Pallet<Runtime>>::block_weight().total(),
				base_block_weight
			);
		});
	}

	#[test]
	fn validate_unsigned() {
		let valid = TestXt::new(RuntimeCall::Custom(CustomCall::allowed_unsigned {}), None);
		let invalid = TestXt::new(RuntimeCall::Custom(CustomCall::unallowed_unsigned {}), None);
		let mut t = new_test_ext(1);

		t.execute_with(|| {
			assert_eq!(
				Executive::validate_transaction(
					TransactionSource::InBlock,
					valid.clone(),
					Default::default(),
				),
				Ok(ValidTransaction::default()),
			);
			assert_eq!(
				Executive::validate_transaction(
					TransactionSource::InBlock,
					invalid.clone(),
					Default::default(),
				),
				Err(TransactionValidityError::Unknown(
					UnknownTransaction::NoUnsignedValidator
				)),
			);
			assert_eq!(
				Executive::apply_extrinsic(valid),
				Ok(Err(DispatchError::BadOrigin))
			);
			assert_eq!(
				Executive::apply_extrinsic(invalid),
				Err(TransactionValidityError::Unknown(
					UnknownTransaction::NoUnsignedValidator
				))
			);
		});
	}

	#[test]
	fn can_pay_for_tx_fee_on_full_lock() {
		let id: LockIdentifier = *b"0       ";
		let execute_with_lock = |lock: WithdrawReasons| {
			let mut t = new_test_ext(1);
			t.execute_with(|| {
				<pallet_balances::Pallet<Runtime> as LockableCurrency<AccountId>>::set_lock(
					id,
					&1,
					111 * AVL,
					lock,
				);
				let call = RuntimeCall::System(SystemCall::remark { remark: vec![1u8] });
				let xt = TestXt::new(call, sign_extra(1, 0, 0));
				let xt_len = xt.encode().len() as Balance;

				let weight = xt.get_dispatch_info().weight
					+ <Runtime as frame_system::Config>::BlockWeights::get()
						.get(DispatchClass::Normal)
						.base_extrinsic;
				let fee: Balance =
					<Runtime as pallet_transaction_payment::Config>::WeightToFee::weight_to_fee(
						&weight,
					);
				Executive::initialize_block(&Header::new(
					1,
					H256::default(),
					H256::default(),
					[69u8; 32].into(),
					Digest::default(),
				));

				if lock == WithdrawReasons::except(WithdrawReasons::TRANSACTION_PAYMENT) {
					assert!(Executive::apply_extrinsic(xt).unwrap().is_ok());
					// tx fee has been deducted.
					assert_eq!(
						<pallet_balances::Pallet<Runtime>>::total_balance(&1),
						111 * AVL - xt_len - fee
					);
				} else {
					assert_eq!(
						Executive::apply_extrinsic(xt),
						Err(InvalidTransaction::Payment.into()),
					);
					assert_eq!(
						<pallet_balances::Pallet<Runtime>>::total_balance(&1),
						111 * AVL
					);
				}
			});
		};

		execute_with_lock(WithdrawReasons::all());
		execute_with_lock(WithdrawReasons::except(
			WithdrawReasons::TRANSACTION_PAYMENT,
		));
	}

	#[test]
	fn block_hooks_weight_is_stored() {
		let header = DaHeader {
			number: 1,
			..Default::default()
		};

		new_test_ext(1).execute_with(|| {
			Executive::initialize_block(&header);
			Executive::finalize_block();
			// NOTE: might need updates over time if new weights are introduced.
			// For now it only accounts for the base block execution weight and
			// the `on_initialize` weight defined in the custom test module.
			assert_eq!(
				<frame_system::Pallet<Runtime>>::block_weight().total(),
				BLOCK_INIT_COST + BLOCK_BASE_COST + BLOCK_IDLE_COST
			);
		})
	}

	#[test]
	fn runtime_upgraded_should_work() {
		new_test_ext(1).execute_with(|| {
			RuntimeVersionTestValues::mutate(|v| *v = Default::default());
			// It should be added at genesis
			assert!(frame_system::LastRuntimeUpgrade::<Runtime>::exists());
			assert!(!Executive::runtime_upgraded());

			RuntimeVersionTestValues::mutate(|v| {
				*v = sp_version::RuntimeVersion {
					spec_version: 1,
					..Default::default()
				}
			});
			assert!(Executive::runtime_upgraded());
			assert_eq!(
				Some(LastRuntimeUpgradeInfo {
					spec_version: 1.into(),
					spec_name: "".into()
				}),
				frame_system::LastRuntimeUpgrade::<Runtime>::get(),
			);

			RuntimeVersionTestValues::mutate(|v| {
				*v = sp_version::RuntimeVersion {
					spec_version: 1,
					spec_name: "test".into(),
					..Default::default()
				}
			});
			assert!(Executive::runtime_upgraded());
			assert_eq!(
				Some(LastRuntimeUpgradeInfo {
					spec_version: 1.into(),
					spec_name: "test".into()
				}),
				frame_system::LastRuntimeUpgrade::<Runtime>::get(),
			);

			RuntimeVersionTestValues::mutate(|v| {
				*v = sp_version::RuntimeVersion {
					spec_version: 1,
					spec_name: "test".into(),
					impl_version: 2,
					..Default::default()
				}
			});
			assert!(!Executive::runtime_upgraded());

			frame_system::LastRuntimeUpgrade::<Runtime>::take();
			assert!(Executive::runtime_upgraded());
			assert_eq!(
				Some(LastRuntimeUpgradeInfo {
					spec_version: 1.into(),
					spec_name: "test".into()
				}),
				frame_system::LastRuntimeUpgrade::<Runtime>::get(),
			);
		})
	}

	#[test]
	fn last_runtime_upgrade_was_upgraded_works() {
		let test_data = vec![
			(0, "", 1, "", true),
			(1, "", 1, "", false),
			(1, "", 1, "test", true),
			(1, "", 0, "", false),
			(1, "", 0, "test", true),
		];

		for (spec_version, spec_name, c_spec_version, c_spec_name, result) in test_data {
			let current = sp_version::RuntimeVersion {
				spec_version: c_spec_version,
				spec_name: c_spec_name.into(),
				..Default::default()
			};

			let last = LastRuntimeUpgradeInfo {
				spec_version: spec_version.into(),
				spec_name: spec_name.into(),
			};

			assert_eq!(result, last.was_upgraded(&current));
		}
	}

	#[test]
	fn custom_runtime_upgrade_is_called_before_modules() {
		new_test_ext(1).execute_with(|| {
			// Make sure `on_runtime_upgrade` is called.
			RuntimeVersionTestValues::mutate(|v| {
				*v = sp_version::RuntimeVersion {
					spec_version: 1,
					..Default::default()
				}
			});

			Executive::initialize_block(&Header::new(
				1,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));

			assert_eq!(&sp_io::storage::get(TEST_KEY).unwrap()[..], *b"module");
			assert_eq!(
				sp_io::storage::get(CUSTOM_ON_RUNTIME_KEY).unwrap(),
				true.encode()
			);
		});
	}

	#[test]
	fn event_from_runtime_upgrade_is_included() {
		new_test_ext(1).execute_with(|| {
			// Make sure `on_runtime_upgrade` is called.
			RuntimeVersionTestValues::mutate(|v| {
				*v = sp_version::RuntimeVersion {
					spec_version: 1,
					..Default::default()
				}
			});

			// set block number to non zero so events are not excluded
			System::set_block_number(1);

			Executive::initialize_block(&Header::new(
				2,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));

			System::assert_last_event(frame_system::Event::<Runtime>::CodeUpdated.into());
		});
	}

	/// Regression test that ensures that the custom on runtime upgrade is called when executive is
	/// used through the `ExecuteBlock` trait.
	#[test]
	fn custom_runtime_upgrade_is_called_when_using_execute_block_trait() {
		let xt = TestXt::new(
			RuntimeCall::Balances(BalancesCall::transfer { dest: 33, value: 0 }),
			sign_extra(1, 0, 0),
		);

		let header = new_test_ext(1).execute_with(|| {
			// Make sure `on_runtime_upgrade` is called.
			RuntimeVersionTestValues::mutate(|v| {
				*v = sp_version::RuntimeVersion {
					spec_version: 1,
					..Default::default()
				}
			});

			// Let's build some fake block.
			Executive::initialize_block(&Header::new(
				1,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));

			Executive::apply_extrinsic(xt.clone()).unwrap().unwrap();

			Executive::finalize_block()
		});

		// Reset to get the correct new genesis below.
		RuntimeVersionTestValues::mutate(|v| {
			*v = sp_version::RuntimeVersion {
				spec_version: 0,
				..Default::default()
			}
		});

		new_test_ext(1).execute_with(|| {
			// Make sure `on_runtime_upgrade` is called.
			RuntimeVersionTestValues::mutate(|v| {
				*v = sp_version::RuntimeVersion {
					spec_version: 1,
					..Default::default()
				}
			});

			<Executive as ExecuteBlock<TestBlock>>::execute_block(Block::new(header, vec![xt]));

			assert_eq!(&sp_io::storage::get(TEST_KEY).unwrap()[..], *b"module");
			assert_eq!(
				sp_io::storage::get(CUSTOM_ON_RUNTIME_KEY).unwrap(),
				true.encode()
			);
		});
	}

	#[test]
	fn all_weights_are_recorded_correctly() {
		new_test_ext(1).execute_with(|| {
			// Make sure `on_runtime_upgrade` is called for maximum complexity
			RuntimeVersionTestValues::mutate(|v| {
				*v = sp_version::RuntimeVersion {
					spec_version: 1,
					..Default::default()
				}
			});

			let block_number = 1;

			Executive::initialize_block(&Header::new(
				block_number,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));

			// All weights that show up in the `initialize_block_impl`
			let frame_system_upgrade_weight = frame_system::Pallet::<Runtime>::on_runtime_upgrade();
			let custom_runtime_upgrade_weight = CustomOnRuntimeUpgrade::on_runtime_upgrade();
			let runtime_upgrade_weight =
				<AllPalletsWithSystem as OnRuntimeUpgrade>::on_runtime_upgrade();
			let frame_system_on_initialize_weight =
				frame_system::Pallet::<Runtime>::on_initialize(block_number);
			let on_initialize_weight =
				<AllPalletsWithSystem as OnInitialize<u32>>::on_initialize(block_number);
			let base_block_weight =
				<Runtime as frame_system::Config>::BlockWeights::get().base_block;

			// Weights are recorded correctly
			assert_eq!(
				frame_system::Pallet::<Runtime>::block_weight().total(),
				frame_system_upgrade_weight
					+ custom_runtime_upgrade_weight
					+ runtime_upgrade_weight
					+ frame_system_on_initialize_weight
					+ on_initialize_weight
					+ base_block_weight,
			);
		});
	}

	#[test]
	fn offchain_worker_works_as_expected() {
		new_test_ext(1).execute_with(|| {
			let parent_hash = sp_core::H256::from([69u8; 32]);
			let mut digest = Digest::default();
			digest.push(DigestItem::Seal([1, 2, 3, 4], vec![5, 6, 7, 8]));

			let header = Header::new(
				1,
				H256::default(),
				H256::default(),
				parent_hash,
				digest.clone(),
			);

			Executive::offchain_worker(&header);

			assert_eq!(digest, System::digest());
			assert_eq!(parent_hash, System::block_hash(0));
			assert_eq!(header.hash(), System::block_hash(1));
		});
	}

	#[test]
	fn calculating_storage_root_twice_works() {
		let call = RuntimeCall::Custom(CustomCall::calculate_storage_root {});
		let xt = TestXt::new(call, sign_extra(1, 0, 0));

		let header = new_test_ext(1).execute_with(|| {
			// Let's build some fake block.
			Executive::initialize_block(&Header::new(
				1,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));

			Executive::apply_extrinsic(xt.clone()).unwrap().unwrap();

			Executive::finalize_block()
		});

		new_test_ext(1).execute_with(|| {
			Executive::execute_block(Block::new(header, vec![xt]));
		});
	}

	#[test]
	#[should_panic(expected = "Invalid inherent position for extrinsic at index 1")]
	fn invalid_inherent_position_fail() {
		let xt1 = TestXt::new(
			RuntimeCall::Balances(BalancesCall::transfer { dest: 33, value: 0 }),
			sign_extra(1, 0, 0),
		);
		let xt2 = TestXt::new(RuntimeCall::Custom(CustomCall::inherent_call {}), None);

		let header = new_test_ext(1).execute_with(|| {
			// Let's build some fake block.
			Executive::initialize_block(&Header::new(
				1,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));

			Executive::apply_extrinsic(xt1.clone()).unwrap().unwrap();
			Executive::apply_extrinsic(xt2.clone()).unwrap().unwrap();

			Executive::finalize_block()
		});

		new_test_ext(1).execute_with(|| {
			Executive::execute_block(Block::new(header, vec![xt1, xt2]));
		});
	}

	#[test]
	fn valid_inherents_position_works() {
		let xt1 = TestXt::new(RuntimeCall::Custom(CustomCall::inherent_call {}), None);
		let xt2 = TestXt::new(call_transfer(33, 0), sign_extra(1, 0, 0));

		let header = new_test_ext(1).execute_with(|| {
			// Let's build some fake block.
			Executive::initialize_block(&Header::new(
				1,
				H256::default(),
				H256::default(),
				[69u8; 32].into(),
				Digest::default(),
			));

			Executive::apply_extrinsic(xt1.clone()).unwrap().unwrap();
			Executive::apply_extrinsic(xt2.clone()).unwrap().unwrap();

			Executive::finalize_block()
		});

		new_test_ext(1).execute_with(|| {
			Executive::execute_block(Block::new(header, vec![xt1, xt2]));
		});
	}
}