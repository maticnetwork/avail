#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]

use crate::{storage_utils::MessageStatusEnum, verifier::Verifier};
use alloy_sol_types::{sol, SolValue};
use ark_std::format;
use avail_base::{MemoryTemporaryStorage, ProvidePostInherent};
use avail_core::data_proof::{tx_uid, AddressedMessage, Message, MessageType};
use sp1_verifier::{Groth16Verifier, GROTH16_VK_BYTES};

use codec::Compact;
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, ExistenceRequirement, UnixTime},
	PalletId,
};
use sp_core::H256;
use sp_runtime::SaturatedConversion;
use sp_std::{vec, vec::Vec};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod constants;
#[cfg(test)]
mod mock;
mod state;
mod storage_utils;
#[cfg(test)]
mod tests;
mod verifier;
mod weights;

pub use pallet::*;

sol! {
	 struct ProofOutputs {
		bytes32 executionStateRoot;
		bytes32 newHeader;
		bytes32 nextSyncCommitteeHash;
		uint256 newHead;
		bytes32 prevHeader;
		uint256 prevHead;
		bytes32 syncCommitteeHash;
	}
}

pub type ProofInput = BoundedVec<u8, ConstU32<1024>>;
pub type PublicValuesInput = BoundedVec<u8, ConstU32<512>>;
pub type FunctionInput = BoundedVec<u8, ConstU32<256>>;
pub type FunctionOutput = BoundedVec<u8, ConstU32<512>>;
pub type FunctionProof = BoundedVec<u8, ConstU32<1048>>;
pub type ValidProof = BoundedVec<BoundedVec<u8, ConstU32<2048>>, ConstU32<32>>;

// Avail asset is supported for now
pub const SUPPORTED_ASSET_ID: H256 = H256::zero();
pub const FAILED_SEND_MSG_ID: &[u8] = b"vector:failed_send_msg_txs";
pub const LOG_TARGET: &str = "runtime::vector";
pub const ROTATE_POSEIDON_OUTPUT_LENGTH: u32 = 32;
pub const STEP_OUTPUT_LENGTH: u32 = 74;

pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use ethabi::Token;
	use ethabi::Token::Uint;
	use frame_support::dispatch::GetDispatchInfo;
	use frame_support::traits::{LockableCurrency, UnfilteredDispatchable};
	use frame_support::{pallet_prelude::ValueQuery, DefaultNoBound};
	use frame_system::pallet_prelude::*;
	use primitive_types::H160;
	use primitive_types::{H256, U256};
	use sp_io::hashing::keccak_256;
	use sp_io::hashing::sha2_256;
	use sp_runtime::traits::AccountIdConversion;
	pub use weights::WeightInfo;

	use crate::state::Configuration;
	use crate::state::{
		parse_rotate_output, parse_step_output, VerifiedRotate, VerifiedStep, VerifiedStepOutput,
	};
	use crate::storage_utils::{get_storage_root, get_storage_value};
	use crate::verifier::encode_packed;

	use super::*;

	#[pallet::error]
	pub enum Error<T> {
		VerificationError,
		NotEnoughParticipants,
		ConfigurationNotSet,
		SlotBehindHead,
		VerificationKeyIsNotSet,
		MalformedVerificationKey,
		FunctionIdNotKnown,
		StepVerificationError,
		RotateVerificationError,
		HeaderRootNotSet,
		VerificationFailed,
		HeaderRootAlreadySet,
		StateRootAlreadySet,
		SyncCommitteeAlreadySet,
		SyncCommitteeNotSet,
		MessageAlreadyExecuted,
		WrongDestinationChain,
		UnsupportedOriginChain,
		BroadcasterSourceChainNotSet,
		SourceChainFrozen,
		CannotGetStorageRoot,
		CannotGetStorageValue,
		InvalidMessageHash,
		CannotDecodeData,
		CannotDecodeDestinationAccountId,
		/// Given AssetId is not supported
		AssetNotSupported,
		/// Given inputs for the selected MessageType are invalid
		InvalidBridgeInputs,
		/// Domain is not supported
		DomainNotSupported,
		/// Function ids (step / rotate) are not set
		FunctionIdsAreNotSet,
		/// Inherent call outside of block execution context.
		BadContext,
		/// Invalid FailedIndices
		InvalidFailedIndices,
		/// Invalid updater
		UpdaterMisMatch,
		/// Proof output parsing error
		CannotParseOutputData,
		/// Cannot get current message id
		CurrentMessageIdNotFound,
		/// Public values decoding error.
		CannotDecodePublicValue,
		/// Sync committee hash is already set for given period.
		SyncCommitteeHashAlreadySet,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Emit event once the head is updated.
		HeadUpdated {
			slot: u64,
			finalization_root: H256,
			execution_state_root: H256,
		},
		/// Emit event once the sync committee updates.
		SyncCommitteeUpdated { period: u64, root: U256 },
		/// Emit when new updater is set.
		BroadcasterUpdated { old: H256, new: H256, domain: u32 },
		/// Emit when message gets executed.
		MessageExecuted {
			from: H256,
			to: H256,
			message_id: u64,
			message_root: H256,
		},
		/// Emit if source chain gets frozen.
		SourceChainFrozen { source_chain_id: u32, frozen: bool },
		/// Emit when message is submitted.
		MessageSubmitted {
			from: T::AccountId,
			to: H256,
			message_type: MessageType,
			destination_domain: u32,
			message_id: u64,
		},
		/// Emit whitelisted domains that are updated.
		WhitelistedDomainsUpdated,
		/// Emit when configuration is updated.
		ConfigurationUpdated {
			slots_per_period: u64,
			finality_threshold: u16,
		},
		/// Emit function Ids that are updated.
		FunctionIdsUpdated { value: Option<(H256, H256)> },
		/// Emit updated step verification key.
		StepVerificationKeyUpdated {
			value: Option<BoundedVec<u8, ConstU32<10_000>>>,
		},
		/// Emit updated rotate verification key.
		RotateVerificationKeyUpdated {
			value: Option<BoundedVec<u8, ConstU32<10_000>>>,
		},
		/// Emit new updater.
		NewUpdater { old: H256, new: H256 },
		/// Emit new SP1 verification key.
		NewSP1VerificationKey { old: H256, new: H256 },
		/// Emit when new sync committee is updated.
		SyncCommitteeHashUpdated { period: u64, hash: H256 },
	}

	/// Storage for a head updates.
	#[pallet::storage]
	#[pallet::getter(fn head)]
	pub type Head<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Maps from a slot to a block header root.
	#[pallet::storage]
	#[pallet::getter(fn headers)]
	pub type Headers<T> = StorageMap<_, Identity, u64, H256, ValueQuery>;

	/// Maps slot to the timestamp of when the headers mapping was updated with slot as a key
	#[pallet::storage]
	pub type Timestamps<T> = StorageMap<_, Identity, u64, u64, ValueQuery>;

	/// Maps from a slot to the current finalized ethereum execution state root.
	#[pallet::storage]
	pub type ExecutionStateRoots<T> = StorageMap<_, Identity, u64, H256, ValueQuery>;

	/// Maps from a period to the poseidon commitment for the sync committee.
	#[pallet::storage]
	#[pallet::getter(fn sync_committee_poseidons)]
	pub type SyncCommitteePoseidons<T> = StorageMap<_, Identity, u64, U256, ValueQuery>;

	/// Storage for a config of finality threshold and slots per period.
	#[pallet::storage]
	pub type ConfigurationStorage<T: Config> = StorageValue<_, Configuration, ValueQuery>;

	/// Maps status of the message to the message root.
	#[pallet::storage]
	pub type MessageStatus<T> = StorageMap<_, Identity, H256, MessageStatusEnum, ValueQuery>;

	/// Mapping between source chainId and the address of the broadcaster on that chain.
	#[pallet::storage]
	pub type Broadcasters<T> = StorageMap<_, Identity, u32, H256, ValueQuery>;

	/// Flags source chain to be frozen.
	#[pallet::storage]
	pub type SourceChainFrozen<T> = StorageMap<_, Identity, u32, bool, ValueQuery>;

	/// List of permitted domains.
	#[pallet::storage]
	pub type WhitelistedDomains<T> = StorageValue<_, BoundedVec<u32, ConstU32<10_000>>, ValueQuery>;

	/// The storage for the step function identifier and the rotate function identifier.
	/// Step function id is used to distinguish step-related functionality within the fulfill_call function.
	/// Rotate function id is used to handle rotate-related functionality within the fulfill_call function.
	/// When the provided function_id matches the step/rotate function identifier, specific logic related to step/rotate functions is executed.
	/// The order of storage is (step_function_id, rotate_function_id)
	#[pallet::storage]
	#[pallet::getter(fn function_ids)]
	pub type FunctionIds<T: Config> = StorageValue<_, Option<(H256, H256)>, ValueQuery>;

	/// Step verification key storage.
	#[pallet::storage]
	#[pallet::getter(fn step_verification_key)]
	pub type StepVerificationKey<T: Config> =
		StorageValue<_, Option<BoundedVec<u8, ConstU32<10_000>>>, ValueQuery>;

	/// Rotate verification key storage.
	#[pallet::storage]
	#[pallet::getter(fn rotate_verification_key)]
	pub type RotateVerificationKey<T: Config> =
		StorageValue<_, Option<BoundedVec<u8, ConstU32<10_000>>>, ValueQuery>;

	/// Genesis validator root, used to check initialization.
	#[pallet::storage]
	#[pallet::getter(fn genesis_validator_root)]
	pub type GenesisValidatorRoot<T: Config> = StorageValue<_, H256, ValueQuery>;

	/// Genesis timestamp, used to check initialization.
	#[pallet::storage]
	#[pallet::getter(fn genesis_timestamp)]
	pub type GenesisTimestamp<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Seconds per slot, used to check initialization.
	#[pallet::storage]
	#[pallet::getter(fn seconds_per_slot)]
	pub type SecondsPerSlot<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Source chain id, used to check initialization.
	#[pallet::storage]
	#[pallet::getter(fn source_chain_id)]
	pub type SourceChainId<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Updater that can submit updates
	#[pallet::storage]
	#[pallet::getter(fn updater)]
	pub type Updater<T: Config> = StorageValue<_, H256, ValueQuery>;

	/// Maps from a period to the the sync committee hash.
	#[pallet::storage]
	#[pallet::getter(fn sync_committee_hashes)]
	pub type SyncCommitteeHashes<T> = StorageMap<_, Identity, u64, H256, ValueQuery>;

	/// SP1 program verification key.
	#[pallet::storage]
	#[pallet::getter(fn sp1_verification_key)]
	pub type SP1VerificationKey<T: Config> = StorageValue<_, H256, ValueQuery>;

	/// Default implementations of [`DefaultConfig`], which can be used to implement [`Config`].
	pub mod config_preludes {
		use super::*;
		use frame_support::derive_impl;
		use frame_support::parameter_types;
		use frame_support::traits::ConstU64;

		parameter_types! {
			pub const BridgePalletId: PalletId = PalletId(*b"avl/brdg");
		}

		/// Provides a viable default config that can be used with
		/// [`derive_impl`](`frame_support::derive_impl`) to derive a testing pallet config
		/// based on this one.
		pub struct TestDefaultConfig;

		#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig, no_aggregated_types
        )]
		impl frame_system::DefaultConfig for TestDefaultConfig {}

		#[frame_support::register_default_impl(TestDefaultConfig)]
		impl DefaultConfig for TestDefaultConfig {
			type WeightInfo = ();
			type MessageMappingStorageIndex = ConstU64<1>;
			type AvailDomain = ConstU32<1>;
			#[inject_runtime_type]
			type RuntimeEvent = ();
			#[inject_runtime_type]
			type RuntimeCall = ();
			type PalletId = BridgePalletId;
		}
	}

	#[pallet::config(with_default)]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		#[pallet::no_default_bounds]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Because this pallet has dispatchables, it depends on the runtime's definition of a call.
		#[pallet::no_default_bounds]
		type RuntimeCall: Parameter
			+ UnfilteredDispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo;
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
		/// Currency type for this pallet.
		#[pallet::no_default]
		type Currency: LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>;
		/// Dependency that can provide current time.
		#[pallet::no_default]
		type TimeProvider: UnixTime;
		/// The index of the `messages` mapping in contract.
		/// This is mandatory when calling execute messages via storage proofs.
		#[pallet::constant]
		type MessageMappingStorageIndex: Get<u64>;
		/// Bridge's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Unique value associated with Avail Network. Used to distinguish messages between Avail and non-Avail networks.
		#[pallet::constant]
		type AvailDomain: Get<u32>;
	}

	#[pallet::genesis_config]
	#[derive(DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub slots_per_period: u64,
		pub finality_threshold: u16,
		pub function_ids: (H256, H256),
		pub sync_committee_poseidon: U256,
		pub period: u64,
		pub broadcaster: H256,
		pub broadcaster_domain: u32,
		pub step_verification_key: Vec<u8>,
		pub rotate_verification_key: Vec<u8>,
		pub whitelisted_domains: Vec<u32>,
		pub genesis_validator_root: H256,
		pub genesis_time: u64,
		pub seconds_per_slot: u64,
		pub source_chain_id: u64,
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			// Preconfigure init data
			<ConfigurationStorage<T>>::put(Configuration {
				slots_per_period: self.slots_per_period,
				finality_threshold: self.finality_threshold,
			});

			let mut domains = self.whitelisted_domains.clone();

			// Whitelisted domains sanitization.
			domains.sort();
			domains.dedup();
			let domains =
				BoundedVec::try_from(domains).expect("Cannot have more than 10_000 domains.");
			WhitelistedDomains::<T>::put(domains);

			Broadcasters::<T>::set(self.broadcaster_domain, self.broadcaster);

			FunctionIds::<T>::set(Some(self.function_ids));

			let step_verification_key = BoundedVec::try_from(self.step_verification_key.clone())
				.expect("Step verification key should be valid at genesis.");
			StepVerificationKey::<T>::set(Some(step_verification_key));

			let rotate_verification_key =
				BoundedVec::try_from(self.rotate_verification_key.clone())
					.expect("Rotate verification key should be valid at genesis.");
			RotateVerificationKey::<T>::set(Some(rotate_verification_key));

			SyncCommitteePoseidons::<T>::insert(self.period, self.sync_committee_poseidon);

			GenesisValidatorRoot::<T>::set(self.genesis_validator_root);

			GenesisTimestamp::<T>::set(self.genesis_time);

			SecondsPerSlot::<T>::set(self.seconds_per_slot);

			SourceChainId::<T>::set(self.source_chain_id);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			if let Some(failed_txs) =
				MemoryTemporaryStorage::take::<Vec<Compact<u32>>>(FAILED_SEND_MSG_ID)
			{
				log::trace!(target: LOG_TARGET, "Failed Txs cleaned: {failed_txs:?}");
			}

			Weight::zero()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		[u8; 32]: From<T::AccountId>,
	{
		/// The entrypoint for fulfilling a call.
		/// function_id Function identifier.
		/// input Function input.
		/// output Function output.
		/// proof  Function proof.
		/// slot  Function slot to update.
		#[pallet::call_index(0)]
		#[pallet::weight(weight_helper::fulfill_call::<T>(* function_id))]
		pub fn fulfill_call(
			origin: OriginFor<T>,
			function_id: H256,
			input: FunctionInput,
			output: FunctionOutput,
			proof: FunctionProof,
			#[pallet::compact] slot: u64,
		) -> DispatchResultWithPostInfo {
			let sender: [u8; 32] = ensure_signed(origin)?.into();
			let updater = Updater::<T>::get();
			// ensure sender is preconfigured
			ensure!(H256(sender) == updater, Error::<T>::UpdaterMisMatch);

			let config = ConfigurationStorage::<T>::get();
			let input_hash = H256(sha2_256(input.as_slice()));
			let output_hash = H256(sha2_256(output.as_slice()));
			let (step_function_id, rotate_function_id) = Self::get_function_ids()?;
			let verifier = Self::get_verifier(function_id, step_function_id, rotate_function_id)?;

			let is_success = verifier
				.verify(input_hash, output_hash, proof.to_vec())
				.map_err(|_| Error::<T>::VerificationError)?;

			// make sure that verification call is valid
			ensure!(is_success, Error::<T>::VerificationFailed);

			// verification is success and, we can safely parse and validate output
			if function_id == step_function_id {
				let step_output = parse_step_output(output.to_vec())
					.map_err(|_| Error::<T>::CannotParseOutputData)?;

				let vs = VerifiedStep::new(function_id, input_hash, step_output);

				if Self::step_into(slot, &config, &vs, step_function_id)? {
					Self::deposit_event(Event::HeadUpdated {
						slot: vs.verified_output.finalized_slot,
						finalization_root: vs.verified_output.finalized_header_root,
						execution_state_root: vs.verified_output.execution_state_root,
					});
				}
			} else if function_id == rotate_function_id {
				let rotate_output = parse_rotate_output(output.to_vec())
					.map_err(|_| Error::<T>::CannotParseOutputData)?;

				let vr = VerifiedRotate::new(function_id, input_hash, rotate_output);

				let period = Self::rotate_into(slot, &config, &vr, rotate_function_id)?;
				Self::deposit_event(Event::SyncCommitteeUpdated {
					period,
					root: vr.sync_committee_poseidon,
				});
			} else {
				return Err(Error::<T>::FunctionIdNotKnown.into());
			}

			Ok(().into())
		}

		/// Executes message if a valid proofs are provided for the supported message type, assets and domains.
		#[pallet::call_index(1)]
		#[pallet::weight({
			match addr_message.message {
				Message::ArbitraryMessage(ref data) => T::WeightInfo::execute_arbitrary_message(data.len() as u32),
				Message::FungibleToken {..} => T::WeightInfo::execute_fungible_token(),
			}
		})]
		pub fn execute(
			origin: OriginFor<T>,
			#[pallet::compact] slot: u64,
			addr_message: AddressedMessage,
			account_proof: ValidProof,
			storage_proof: ValidProof,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			let encoded_data = addr_message.clone().abi_encode();
			let message_root = H256(keccak_256(encoded_data.as_slice()));

			Self::check_preconditions(&addr_message, message_root)?;

			ensure!(
				!SourceChainFrozen::<T>::get(addr_message.origin_domain),
				Error::<T>::SourceChainFrozen
			);
			let root = ExecutionStateRoots::<T>::get(slot);
			let broadcaster = Broadcasters::<T>::get(addr_message.origin_domain);

			// extract contract address
			let contract_broadcaster_address = H160::from_slice(broadcaster[..20].as_ref());
			let account_proof_vec = account_proof
				.iter()
				.map(|inner_bounded_vec| inner_bounded_vec.iter().copied().collect())
				.collect();

			let storage_root =
				get_storage_root(account_proof_vec, contract_broadcaster_address, root)
					.map_err(|_| Error::<T>::CannotGetStorageRoot)?;

			let message_id = Uint(U256::from(addr_message.id));
			let mm_idx = Uint(U256::from(T::MessageMappingStorageIndex::get()));
			let slot_key = H256(keccak_256(ethabi::encode(&[message_id, mm_idx]).as_slice()));

			let storage_proof_vec = storage_proof
				.iter()
				.map(|inner_bounded_vec| inner_bounded_vec.iter().copied().collect())
				.collect();

			let slot_value = get_storage_value(slot_key, storage_root, storage_proof_vec)
				.map_err(|_| Error::<T>::CannotGetStorageValue)?;

			ensure!(slot_value == message_root, Error::<T>::InvalidMessageHash);

			if let Message::FungibleToken { asset_id, amount } = &addr_message.message {
				ensure!(
					SUPPORTED_ASSET_ID == *asset_id,
					Error::<T>::AssetNotSupported
				);

				let destination_account_id =
					T::AccountId::decode(&mut &addr_message.to.encode()[..])
						.map_err(|_| Error::<T>::CannotDecodeDestinationAccountId)?;

				T::Currency::transfer(
					&Self::account_id(),
					&destination_account_id,
					(*amount).saturated_into(),
					ExistenceRequirement::AllowDeath,
				)?;
			}

			MessageStatus::<T>::set(message_root, MessageStatusEnum::ExecutionSucceeded);
			Self::deposit_event(Event::<T>::MessageExecuted {
				from: addr_message.from,
				to: addr_message.to,
				message_id: addr_message.id,
				message_root,
			});

			Ok(().into())
		}

		/// source_chain_froze froze source chain and prevent messages to be executed.
		//
		// Test names: source_chain_froze_works_with_root(), source_chain_froze_does_not_work_with_non_root()
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::source_chain_froze())]
		pub fn source_chain_froze(
			origin: OriginFor<T>,
			#[pallet::compact] source_chain_id: u32,
			frozen: bool,
		) -> DispatchResult {
			ensure_root(origin)?;

			SourceChainFrozen::<T>::set(source_chain_id, frozen);
			Self::deposit_event(Event::<T>::SourceChainFrozen {
				source_chain_id,
				frozen,
			});

			Ok(())
		}

		/// send_message sends a message from an origin chain to the destination chain.
		//
		// Test names:
		//	send_message_fungible_token_works(), send_message_fungible_token_doesnt_accept_data(),
		//	send_message_fungible_token_doesnt_accept_empty_asset_id(), send_message_fungible_token_doesnt_accept_empty_value(),
		//	send_message_arbitrary_message_works(), send_message_arbitrary_message_doesnt_accept_value(),
		//	send_message_arbitrary_message_doesnt_accept_asset_id(), send_message_arbitrary_message_doesnt_accept_empty_data()
		#[pallet::call_index(3)]
		#[pallet::weight({
			match message {
				Message::ArbitraryMessage(ref data) => T::WeightInfo::send_message_arbitrary_message(data.len() as u32),
				Message::FungibleToken{..} => T::WeightInfo::send_message_fungible_token(),
			}
		})]
		pub fn send_message(
			origin: OriginFor<T>,
			message: Message,
			to: H256,
			#[pallet::compact] domain: u32,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let dispatch = Self::do_send_message(who, message, to, domain);
			if dispatch.is_err() {
				let _ = MemoryTemporaryStorage::update::<Vec<Compact<u32>>, _>(
					FAILED_SEND_MSG_ID.to_vec(),
					|failed| {
						let tx_idx_result = <frame_system::Pallet<T>>::extrinsic_index();
						// this should never happen and we can just log warn
						if tx_idx_result.is_none() {
							log::warn!(target: LOG_TARGET, "Transaction index is none!");
						}
						let tx_idx = tx_idx_result.unwrap_or_default();
						failed.push(tx_idx.into());
						log::trace!(target: LOG_TARGET, "Send Message failed txs: {failed:?}");
					},
				);
			}

			dispatch
		}

		/// set_poseidon_hash sets poseidon hash of the sync committee for the particular period.
		//
		// Test names: set_poseidon_hash_works_with_root(), set_poseidon_hash_does_not_work_with_non_root()
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::set_poseidon_hash())]
		pub fn set_poseidon_hash(
			origin: OriginFor<T>,
			#[pallet::compact] period: u64,
			poseidon_hash: BoundedVec<u8, ConstU32<200>>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			// poseidon_hash.len() is always less than `u32::MAX` because it is bounded by BoundedVec
			ensure!(
				poseidon_hash.len() as u32 <= ROTATE_POSEIDON_OUTPUT_LENGTH,
				Error::<T>::CannotParseOutputData
			);

			let hash = U256::from(poseidon_hash.to_vec().as_slice());

			SyncCommitteePoseidons::<T>::insert(period, hash);
			Self::deposit_event(Event::SyncCommitteeUpdated { period, root: hash });
			Ok(().into())
		}

		/// set_broadcaster sets the broadcaster address of the message from the origin chain.
		//
		// Test names: set_broadcaster_works_with_root(), set_broadcaster_does_not_work_with_non_root()
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::set_broadcaster())]
		pub fn set_broadcaster(
			origin: OriginFor<T>,
			#[pallet::compact] broadcaster_domain: u32,
			broadcaster: H256,
		) -> DispatchResult {
			ensure_root(origin)?;
			let old_bc = Broadcasters::<T>::get(broadcaster_domain);

			Broadcasters::<T>::set(broadcaster_domain, broadcaster);

			Self::deposit_event(Event::BroadcasterUpdated {
				old: old_bc,
				new: broadcaster,
				domain: broadcaster_domain,
			});

			Ok(())
		}

		/// The set_whitelisted_domains function allows the root (administrator) to set the whitelisted domains. It is a
		/// privileged function intended for administrative purposes, used to manage a list of permitted domains.
		//
		// Test names: set_whitelisted_domains_works_with_root(), set_whitelisted_domains_does_not_work_with_non_root()
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::set_whitelisted_domains())]
		pub fn set_whitelisted_domains(
			origin: OriginFor<T>,
			value: BoundedVec<u32, ConstU32<10_000>>,
		) -> DispatchResult {
			ensure_root(origin)?;
			WhitelistedDomains::<T>::put(value);

			Self::deposit_event(Event::WhitelistedDomainsUpdated);

			Ok(())
		}

		/// The set_configuration function allows the root (administrator) to set the configuration. It is a
		/// privileged function intended for administrative purposes, used to manage slots_per_period and finality_threshold values.
		//
		// Test names: set_configuration_works_with_root(), set_configuration_does_not_work_with_non_root()
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::set_configuration())]
		pub fn set_configuration(origin: OriginFor<T>, value: Configuration) -> DispatchResult {
			ensure_root(origin)?;
			ConfigurationStorage::<T>::put(value);

			Self::deposit_event(Event::ConfigurationUpdated {
				slots_per_period: value.slots_per_period,
				finality_threshold: value.finality_threshold,
			});

			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::set_function_ids())]
		pub fn set_function_ids(
			origin: OriginFor<T>,
			value: Option<(H256, H256)>,
		) -> DispatchResult {
			ensure_root(origin)?;
			FunctionIds::<T>::put(value);

			Self::deposit_event(Event::FunctionIdsUpdated { value });

			Ok(())
		}

		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::set_step_verification_key())]
		pub fn set_step_verification_key(
			origin: OriginFor<T>,
			value: Option<BoundedVec<u8, ConstU32<10_000>>>,
		) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(vk) = value.clone() {
				let _ = Verifier::from_json_u8_slice(vk.as_slice())
					.map_err(|_| Error::<T>::MalformedVerificationKey)?;
			}

			StepVerificationKey::<T>::put(value.clone());

			Self::deposit_event(Event::StepVerificationKeyUpdated { value });

			Ok(())
		}

		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::set_rotate_verification_key())]
		pub fn set_rotate_verification_key(
			origin: OriginFor<T>,
			value: Option<BoundedVec<u8, ConstU32<10_000>>>,
		) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(vk) = value.clone() {
				let _ = Verifier::from_json_u8_slice(vk.as_slice())
					.map_err(|_| Error::<T>::MalformedVerificationKey)?;
			}

			RotateVerificationKey::<T>::put(value.clone());

			Self::deposit_event(Event::RotateVerificationKeyUpdated { value });

			Ok(())
		}

		#[pallet::call_index(11)]
		#[pallet::weight((
			T::WeightInfo::failed_tx_index(0u32),
			DispatchClass::Mandatory
		))]
		pub fn failed_send_message_txs(
			origin: OriginFor<T>,
			failed_txs: Vec<Compact<u32>>,
		) -> DispatchResult {
			ensure_none(origin)?;
			let local_failed_txs =
				MemoryTemporaryStorage::get::<Vec<Compact<u32>>>(FAILED_SEND_MSG_ID)
					.unwrap_or_default();
			ensure!(
				local_failed_txs == failed_txs,
				Error::<T>::InvalidFailedIndices
			);

			Ok(())
		}

		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::set_updater())]
		pub fn set_updater(origin: OriginFor<T>, updater: H256) -> DispatchResult {
			ensure_root(origin)?;
			let old = Updater::<T>::get();
			Updater::<T>::set(updater);

			Self::deposit_event(Event::<T>::NewUpdater { old, new: updater });
			Ok(())
		}

		/// The entrypoint for fulfill a call.
		/// proof Input proof value.
		/// public_values Input public values.
		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::fulfill())]
		pub fn fulfill(
			origin: OriginFor<T>,
			proof: ProofInput,
			public_values: PublicValuesInput,
		) -> DispatchResultWithPostInfo {
			let sender: [u8; 32] = ensure_signed(origin)?.into();
			let updater = Updater::<T>::get();
			// ensure sender is preconfigured
			ensure!(H256(sender) == updater, Error::<T>::UpdaterMisMatch);

			let proof_outputs: ProofOutputs = SolValue::abi_decode(&public_values, true)
				.map_err(|_| Error::<T>::CannotDecodePublicValue)?;

			let head = Head::<T>::get();
			let new_head: u64 = proof_outputs.newHead.to();
			ensure!(new_head > head, Error::<T>::SlotBehindHead);

			let sp1_vk = SP1VerificationKey::<T>::get();

			// Can throw panic in the sp1 v3.4.0 library if the proof is not valid in some cases
			// and, it will be fixed in sp1 version v4.0.0
			let is_valid = Groth16Verifier::verify(
				&proof,
				&public_values,
				&format!("{:?}", sp1_vk),
				&GROTH16_VK_BYTES,
			);
			ensure!(is_valid.is_ok(), Error::<T>::VerificationFailed);

			Head::<T>::set(new_head);
			let header = Headers::<T>::get(new_head);
			ensure!(header == H256::zero(), Error::<T>::HeaderRootAlreadySet);

			let new_header = H256::from(proof_outputs.newHeader.0);

			let execution_state_root = ExecutionStateRoots::<T>::get(new_head);
			ensure!(
				execution_state_root == H256::zero(),
				Error::<T>::StateRootAlreadySet
			);
			let new_execution_state_root = H256::from(proof_outputs.executionStateRoot.0);

			Headers::<T>::insert(new_head, new_header);
			ExecutionStateRoots::<T>::insert(new_head, new_execution_state_root);

			Self::deposit_event(Event::HeadUpdated {
				slot: new_head,
				finalization_root: new_header,
				execution_state_root: new_execution_state_root,
			});

			let config = ConfigurationStorage::<T>::get();
			let period = new_head
				.checked_div(config.slots_per_period)
				.ok_or(Error::<T>::ConfigurationNotSet)?;

			// If the sync committee for the period is not set, set it.
			// This can happen if the light client was very behind and had a lot of updates
			// and only the last sync committee is stored, not the intermediate ones for every period and may have gaps in periods.
			if SyncCommitteeHashes::<T>::get(period) == H256::zero() {
				let sync_committee_hash = H256::from(proof_outputs.syncCommitteeHash.0);
				SyncCommitteeHashes::<T>::set(period, sync_committee_hash);
				Self::deposit_event(Event::SyncCommitteeHashUpdated {
					period,
					hash: sync_committee_hash,
				});
			}

			// Update next sync committee hash only if it is not set
			let next_sync_committee_hash = H256::from(proof_outputs.nextSyncCommitteeHash.0);
			if next_sync_committee_hash != H256::zero() {
				let next_period = period + 1;

				let sync_committee_hash = SyncCommitteeHashes::<T>::get(next_period);
				if sync_committee_hash != next_sync_committee_hash {
					ensure!(
						sync_committee_hash == H256::zero(),
						Error::<T>::SyncCommitteeHashAlreadySet
					);
					SyncCommitteeHashes::<T>::set(next_period, next_sync_committee_hash);
					Self::deposit_event(Event::SyncCommitteeHashUpdated {
						period: next_period,
						hash: next_sync_committee_hash,
					});
				}
			}
			Timestamps::<T>::insert(new_head, T::TimeProvider::now().as_secs());

			Ok(().into())
		}

		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::set_sp1_verification_key())]
		pub fn set_sp1_verification_key(origin: OriginFor<T>, sp1_vk: H256) -> DispatchResult {
			ensure_root(origin)?;
			let old_vk = SP1VerificationKey::<T>::get();
			SP1VerificationKey::<T>::put(sp1_vk);

			Self::deposit_event(Event::NewSP1VerificationKey {
				old: old_vk,
				new: sp1_vk,
			});

			Ok(())
		}

		#[pallet::call_index(15)]
		#[pallet::weight(T::WeightInfo::set_sync_committee_hash())]
		pub fn set_sync_committee_hash(
			origin: OriginFor<T>,
			period: u64,
			hash: H256,
		) -> DispatchResult {
			ensure_root(origin)?;

			SyncCommitteeHashes::<T>::insert(period, hash);
			Self::deposit_event(Event::SyncCommitteeHashUpdated { period, hash });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn do_send_message(
			who: T::AccountId,
			message: Message,
			to: H256,
			domain: u32,
		) -> DispatchResultWithPostInfo {
			// Ensure the domain is currently supported
			ensure!(
				Self::is_domain_valid(domain),
				Error::<T>::DomainNotSupported
			);
			// Check MessageType and enforce the rules
			let message_type = message.r#type();
			match message {
				Message::FungibleToken { asset_id, amount } => {
					ensure!(
						SUPPORTED_ASSET_ID == asset_id,
						Error::<T>::AssetNotSupported
					);
					ensure!(
						amount.saturated_into::<u128>() > 0,
						Error::<T>::InvalidBridgeInputs
					);
					T::Currency::transfer(
						&who,
						&Self::account_id(),
						amount.saturated_into(),
						ExistenceRequirement::KeepAlive,
					)?;
				},
				Message::ArbitraryMessage(data) => {
					ensure!(!data.is_empty(), Error::<T>::InvalidBridgeInputs)
				},
			};

			let message_id = Self::fetch_curr_message_id()?;

			Self::deposit_event(Event::MessageSubmitted {
				from: who,
				to,
				message_type,
				destination_domain: domain,
				message_id,
			});

			Ok(().into())
		}

		fn fetch_curr_message_id() -> Result<u64, DispatchError> {
			let number = <frame_system::Pallet<T>>::block_number().saturated_into::<u32>();
			let tx_index_option = <frame_system::Pallet<T>>::extrinsic_index();

			match tx_index_option {
				Some(tx_index) => Ok(tx_uid(number, tx_index)),
				None => Err(Error::<T>::CurrentMessageIdNotFound.into()),
			}
		}

		fn check_preconditions(
			message: &AddressedMessage,
			message_root: H256,
		) -> Result<(), DispatchError> {
			let message_status = MessageStatus::<T>::get(message_root);
			// Message must not be executed
			ensure!(
				message_status == MessageStatusEnum::NotExecuted,
				Error::<T>::MessageAlreadyExecuted
			);

			ensure!(
				message.destination_domain == T::AvailDomain::get(),
				Error::<T>::WrongDestinationChain
			);

			ensure!(
				WhitelistedDomains::<T>::get().contains(&message.origin_domain),
				Error::<T>::UnsupportedOriginChain
			);

			let source_chain = Broadcasters::<T>::get(message.origin_domain);
			ensure!(
				source_chain != H256::zero(),
				Error::<T>::BroadcasterSourceChainNotSet
			);

			Ok(())
		}

		/// The account ID of the bridge's pot.
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		fn rotate_into(
			finalized_slot: u64,
			cfg: &Configuration,
			verified_rotate_call: &VerifiedRotate,
			rotate_function_id: H256,
		) -> Result<u64, DispatchError> {
			let finalized_header_root = Headers::<T>::get(finalized_slot);
			ensure!(
				finalized_header_root != H256::zero(),
				Error::<T>::HeaderRootNotSet
			);

			let input = ethabi::encode(&[Token::FixedBytes(finalized_header_root.0.to_vec())]);
			let sync_committee_poseidon: U256 =
				Self::verified_rotate_call(rotate_function_id, input, verified_rotate_call)?;

			let period = finalized_slot
				.checked_div(cfg.slots_per_period)
				.ok_or(Error::<T>::ConfigurationNotSet)?;
			let next_period = period + 1;

			Self::set_sync_committee_poseidon(next_period, sync_committee_poseidon)?;

			Ok(next_period)
		}

		fn step_into(
			attested_slot: u64,
			cfg: &Configuration,
			verified_step_call: &VerifiedStep,
			step_function_id: H256,
		) -> Result<bool, DispatchError> {
			let period = attested_slot
				.checked_div(cfg.slots_per_period)
				.ok_or(Error::<T>::ConfigurationNotSet)?;

			let sc_poseidon = SyncCommitteePoseidons::<T>::get(period);
			ensure!(sc_poseidon != U256::zero(), Error::<T>::SyncCommitteeNotSet);

			let input = encode_packed(sc_poseidon, attested_slot);
			let result = Self::verified_step_call(step_function_id, input, verified_step_call)?;
			ensure!(
				result.participation >= cfg.finality_threshold,
				Error::<T>::NotEnoughParticipants
			);

			let head = Head::<T>::get();
			ensure!(result.finalized_slot > head, Error::<T>::SlotBehindHead);

			let updated = Self::set_slot_roots(result)?;

			Ok(updated)
		}

		///  Sets the current slot for the chain the light client is reflecting.
		/// checks is the roots exists for the slot already. If there is
		/// an existing header but no conflict, do nothing. Avoids timestamp renewal DoS attacks.
		fn set_slot_roots(step_output: VerifiedStepOutput) -> Result<bool, DispatchError> {
			let header = Headers::<T>::get(step_output.finalized_slot);
			ensure!(header == H256::zero(), Error::<T>::HeaderRootAlreadySet);

			let execution_state_root = ExecutionStateRoots::<T>::get(step_output.finalized_slot);
			ensure!(
				execution_state_root == H256::zero(),
				Error::<T>::StateRootAlreadySet
			);

			Head::<T>::set(step_output.finalized_slot);
			Headers::<T>::insert(
				step_output.finalized_slot,
				step_output.finalized_header_root,
			);
			ExecutionStateRoots::<T>::insert(
				step_output.finalized_slot,
				step_output.execution_state_root,
			);

			Timestamps::<T>::insert(step_output.finalized_slot, T::TimeProvider::now().as_secs());

			Ok(true)
		}

		/// Sets the sync committee poseidon for a given period.
		fn set_sync_committee_poseidon(period: u64, poseidon: U256) -> Result<(), DispatchError> {
			let sync_committee_poseidons = SyncCommitteePoseidons::<T>::get(period);
			ensure!(
				sync_committee_poseidons == U256::zero(),
				Error::<T>::SyncCommitteeAlreadySet
			);

			SyncCommitteePoseidons::<T>::set(period, poseidon);

			Ok(())
		}

		/// get_verifier returns verifier based on the provided function id.
		fn get_verifier(
			function_id: H256,
			step_function_id: H256,
			rotate_function_id: H256,
		) -> Result<Verifier, Error<T>> {
			if function_id == step_function_id {
				Self::get_step_verifier()
			} else if function_id == rotate_function_id {
				Self::get_rotate_verifier()
			} else {
				Err(Error::<T>::FunctionIdNotKnown)
			}
		}

		fn get_step_verifier() -> Result<Verifier, Error<T>> {
			if let Some(vk) = StepVerificationKey::<T>::get() {
				let deserialized_vk = Verifier::from_json_u8_slice(vk.as_slice())
					.map_err(|_| Error::<T>::MalformedVerificationKey)?;
				Ok(deserialized_vk)
			} else {
				Err(Error::<T>::VerificationKeyIsNotSet)
			}
		}

		fn get_rotate_verifier() -> Result<Verifier, Error<T>> {
			if let Some(vk) = RotateVerificationKey::<T>::get() {
				let deserialized_vk = Verifier::from_json_u8_slice(vk.as_slice())
					.map_err(|_| Error::<T>::MalformedVerificationKey)?;
				Ok(deserialized_vk)
			} else {
				Err(Error::<T>::VerificationKeyIsNotSet)
			}
		}

		fn verified_step_call(
			function_id: H256,
			input: ethabi::Bytes,
			verified_call: &VerifiedStep,
		) -> Result<VerifiedStepOutput, DispatchError> {
			let input_hash = sha2_256(input.as_slice());

			if verified_call.verified_function_id == function_id
				&& verified_call.verified_input_hash == H256(input_hash)
			{
				let verified_output: VerifiedStepOutput = verified_call.verified_output;
				Ok(verified_output)
			} else {
				Err(Error::<T>::StepVerificationError.into())
			}
		}

		fn verified_rotate_call(
			function_id: H256,
			input: ethabi::Bytes,
			verified_call: &VerifiedRotate,
		) -> Result<U256, DispatchError> {
			let input_hash = sha2_256(input.as_slice());

			if verified_call.verified_function_id == function_id
				&& verified_call.verified_input_hash == H256(input_hash)
			{
				Ok(verified_call.sync_committee_poseidon)
			} else {
				Err(Error::<T>::RotateVerificationError.into())
			}
		}

		/// Check if the given domain is supported or not
		fn is_domain_valid(domain: u32) -> bool {
			WhitelistedDomains::<T>::get().contains(&domain)
		}

		fn get_function_ids() -> Result<(H256, H256), DispatchError> {
			if let Some(function_ids) = FunctionIds::<T>::get() {
				Ok(function_ids)
			} else {
				Err(Error::<T>::FunctionIdsAreNotSet.into())
			}
		}
	}
}

impl<T: Config> ProvidePostInherent for Pallet<T>
where
	[u8; 32]: From<T::AccountId>,
{
	type Call = Call<T>;
	type Error = ();

	fn create_inherent(_: &avail_base::StorageMap) -> Option<Self::Call> {
		let failed_txs = MemoryTemporaryStorage::get::<Vec<Compact<u32>>>(FAILED_SEND_MSG_ID)
			.unwrap_or_default();

		log::trace!(target: LOG_TARGET, "Create post inherent failed vector txs: {failed_txs:?}");
		Some(Call::failed_send_message_txs { failed_txs })
	}

	fn is_inherent(call: &Self::Call) -> bool {
		matches!(call, Call::failed_send_message_txs { .. })
	}

	fn check_inherent(call: &Self::Call) -> Result<(), Self::Error> {
		if let Call::failed_send_message_txs { failed_txs } = call {
			let local_failed_txs =
				MemoryTemporaryStorage::get::<Vec<Compact<u32>>>(FAILED_SEND_MSG_ID)
					.unwrap_or_default();
			ensure!(&local_failed_txs == failed_txs, ());
		}
		Ok(())
	}
}

pub mod weight_helper {
	use super::*;

	/// Weight for `dataAvailability::submit_data`.
	pub fn fulfill_call<T: Config>(function_id: H256) -> (Weight, DispatchClass) {
		if let Some((step_function_id, _)) = FunctionIds::<T>::get() {
			if step_function_id == function_id {
				return (T::WeightInfo::fulfill_call_step(), DispatchClass::Normal);
			}
		}
		(T::WeightInfo::fulfill_call_rotate(), DispatchClass::Normal)
	}
}
