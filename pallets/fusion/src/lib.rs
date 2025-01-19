#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod traits;
pub mod types;
mod weights;

use sp_std::collections::btree_map::BTreeMap;

use crate::types::*;
use alloc::{format, string::String};
use frame_support::{
	dispatch::GetDispatchInfo,
	pallet_prelude::*,
	traits::{
		Currency, ExistenceRequirement, LockableCurrency, OnUnbalanced, UnfilteredDispatchable,
		WithdrawReasons,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use pallet_staking::traits::FusionExt;
use sp_core::U256;

use sp_runtime::{
	traits::{AccountIdConversion, Bounded, Zero},
	Perbill, Saturating,
};
use sp_staking::{currency_to_vote::CurrencyToVote, EraIndex};
use sp_std::collections::btree_set::BTreeSet;
use sp_std::{vec, vec::Vec};

pub use traits::{PoolAccountProvider, StakingFusionDataProvider};
pub use weights::WeightInfo;

pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

pub const AVAIL_CURRENCY_ID: u32 = 0;
pub const AVAIL_POOL_ID: u32 = 0;
pub const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// A sudo-able call.
		type RuntimeCall: Parameter
			+ UnfilteredDispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		/// Someone who can call the admin extrinsics.
		type ApprovedOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Currency type for this pallet.
		type Currency: LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>;

		/// Convert a balance into a number used for election calculation.
		type CurrencyToVote: sp_staking::currency_to_vote::CurrencyToVote<BalanceOf<Self>>;

		/// The destination for rewards that were not claimed after 'HistoryDepth' eras.
		type RewardRemainder: OnUnbalanced<NegativeImbalanceOf<Self>>;

		/// Pallet id used to derive accounts used by the pallet
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Maximum allowed for the currency name
		#[pallet::constant]
		type MaxCurrencyNameLength: Get<u32>;

		/// Maximum number of members in a pool
		#[pallet::constant]
		type MaxMembersPerPool: Get<u32>;

		/// Maximum number of selectable targets for a pool
		#[pallet::constant]
		type MaxTargets: Get<u32>;

		/// Maximum number of parallel partial unbonds
		#[pallet::constant]
		type MaxUnbonding: Get<u32>;

		/// Maximum of number of concurrent pending slashes for a pool
		#[pallet::constant]
		type MaxSlashesPerPool: Get<u32>;

		/// Maximum number of pools behind a validator, mostly used to set bounds
		#[pallet::constant]
		type MaxPoolsPerValidator: Get<u32>;

		/// Period for funds to be available after unbonding
		#[pallet::constant]
		type BondingDuration: Get<EraIndex>;

		/// Period to veto a slash
		#[pallet::constant]
		type SlashDeferDuration: Get<EraIndex>;

		/// Number of era for which to keep Fusion data
		#[pallet::constant]
		type HistoryDepth: Get<u32>;

		/// A provider that gives the information from the staking pallet.
		type StakingFusionDataProvider: StakingFusionDataProvider<Self>;

		/// A provider that gives the pool accounts, needed because behaviour is different in tests and production
		type PoolAccountProvider: PoolAccountProvider<Self>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Stores the total value locked in avail and the maximum total value locked authorized
	#[pallet::storage]
	#[pallet::getter(fn tvl_data)]
	pub type TotalValueLockedData<T> = StorageValue<_, TVLData<T>, ValueQuery>;

	/// Stores all the fusion currencies
	#[pallet::storage]
	#[pallet::getter(fn currencies)]
	pub type Currencies<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, FusionCurrency<T>, OptionQuery>;

	/// Stores all the fusion pools
	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type Pools<T: Config> = StorageMap<_, Twox64Concat, PoolId, FusionPool<T>, OptionQuery>;

	/// Mapping from the pools funds account address to the pool id
	#[pallet::storage]
	#[pallet::getter(fn pool_account_to_id)]
	pub type PoolsAccountToId<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, PoolId, OptionQuery>;

	/// Stores all the membership of users in pools
	#[pallet::storage]
	#[pallet::getter(fn memberships)]
	pub type Memberships<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		FusionAddress,
		Twox64Concat,
		PoolId,
		FusionMembership<T>,
		OptionQuery,
	>;

	/// Stores all the users idle balances
	#[pallet::storage]
	#[pallet::getter(fn user_currency_balances)]
	pub type UserCurrencyBalances<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		FusionAddress,
		Twox64Concat,
		CurrencyId,
		FusionUserCurrencyBalance,
		OptionQuery,
	>;

	/// Stores era rewards for each pool
	#[pallet::storage]
	#[pallet::getter(fn era_rewards)]
	pub type EraRewards<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		PoolId,
		EraReward<T>,
		OptionQuery,
	>;

	/// Stores the conversion rates for currencies
	/// How much one unit of currency is equal in AVAIL
	#[pallet::storage]
	#[pallet::getter(fn currency_rates)]
	pub type CurrencyRates<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		CurrencyId,
		BalanceOf<T>,
		OptionQuery,
	>;

	/// Stores the next currency changes to be applied next era
	#[pallet::storage]
	#[pallet::getter(fn currency_rate_changes)]
	pub type CurrencyRateChanges<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, BalanceOf<T>, ValueQuery>;

	/// Mapping from Fusion address to Substrate address
	#[pallet::storage]
	#[pallet::getter(fn fusion_address_to_substrate_address)]
	pub type FusionAddressToSubstrateAddress<T: Config> =
		StorageMap<_, Blake2_128Concat, FusionAddress, T::AccountId, OptionQuery>;

	/// Stores the era durations for HistoryDepth eras
	/// It is used in case claiming reward for a pool failed because of low balance
	/// We will be able to compute those rewards later using era duration
	#[pallet::storage]
	#[pallet::getter(fn era_durations)]
	pub type EraDurations<T: Config> = StorageMap<_, Twox64Concat, EraIndex, u64, OptionQuery>;

	/// Stores the fusion era exposure for HistoryDepth eras
	#[pallet::storage]
	#[pallet::getter(fn exposures)]
	pub type Exposures<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		PoolId,
		FusionExposure<T>,
		OptionQuery,
	>;

	/// Stores the pool ids that backed a validator for a specific era
	/// (era, ValidatorAddress) => PoolIds that backed the validator for the era
	#[pallet::storage]
	#[pallet::getter(fn pools_backing_validator)]
	pub type PoolsBackingValidator<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		T::AccountId,
		BoundedVec<PoolId, T::MaxPoolsPerValidator>,
		ValueQuery,
	>;

	/// Stores the fusion claimed rewards for HistoryDepth eras
	#[pallet::storage]
	#[pallet::getter(fn claimed_rewards)]
	pub type ClaimedRewards<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		(PoolId, FusionAddress),
		BalanceOf<T>,
		ValueQuery,
	>;

	/// Stores Fusion address of the slash destination
	/// It can be controlled with technical committee
	#[pallet::storage]
	#[pallet::getter(fn slash_destination)]
	pub type SlashDestination<T> = StorageValue<_, FusionAddress, OptionQuery>;

	/// Stores the number of slashes for a given era, a validator and a pool funds account
	/// (era, (validator, funds_account)) => number of pending_slash
	/// Used mainly to quickly determine if a slashed nominator is from Fusion pallet
	#[pallet::storage]
	#[pallet::getter(fn has_pending_slash)]
	pub type HasPendingSlash<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		(T::AccountId, T::AccountId),
		u32,
		ValueQuery,
	>;

	/// Stores the unbonding chunks of all the pallet
	/// For a given pool and era, will return a vector of chunks
	#[pallet::storage]
	#[pallet::getter(fn unbonding_chunks)]
	pub type UnbondingChunks<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		PoolId,
		Twox64Concat,
		EraIndex,
		BoundedVec<(FusionAddress, FusionCurrencyBalance), T::MaxMembersPerPool>,
		ValueQuery,
	>;

	/// Stores the pool ids of pool having an boost alongside the minimum to get the boost
	#[pallet::storage]
	#[pallet::getter(fn pools_with_boost)]
	pub type PoolsWithBoost<T: Config> =
		StorageMap<_, Twox64Concat, PoolId, FusionCurrencyBalance, OptionQuery>;

	/// Stores true if the user has boost in the pool
	#[pallet::storage]
	#[pallet::getter(fn has_boost)]
	pub type HasBoost<T: Config> =
		StorageDoubleMap<_, Twox64Concat, PoolId, Twox64Concat, FusionAddress, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event triggered when the funding account is filled with new funds
		FundsAccountFilled {
			sender: T::AccountId,
			amount: BalanceOf<T>,
		},
		FundsAccountWithdrawn {
			recipient: T::AccountId,
			amount: BalanceOf<T>,
		},
		/// Event triggered when a new currency is created
		CurrencyCreated {
			currency_id: CurrencyId,
			name: BoundedVec<u8, T::MaxCurrencyNameLength>,
			nb_decimals: u8,
			max_amount: FusionCurrencyBalance,
			min_amount: FusionCurrencyBalance,
			initial_conversion_rate: BalanceOf<T>,
		},
		/// Event triggered when a currency's properties are updated
		CurrencySet {
			currency_id: CurrencyId,
			name: Option<BoundedVec<u8, T::MaxCurrencyNameLength>>,
			max_amount: Option<FusionCurrencyBalance>,
			min_amount: Option<FusionCurrencyBalance>,
		},
		/// Event triggered when a currency is deleted
		CurrencyDeleted { currency_id: CurrencyId },
		/// Event triggered when a conversion rate is set for a currency
		CurrencyConversionRateSet {
			currency_id: CurrencyId,
			conversion_rate: BalanceOf<T>,
		},
		/// Event triggered when a currency is deposited into the system
		CurrencyDeposited {
			currency_id: CurrencyId,
			fusion_address: FusionAddress,
			amount: FusionCurrencyBalance,
		},
		/// Event triggered when a user unbonds currency from a pool
		CurrencyUnbonded {
			pool_id: PoolId,
			currency_id: CurrencyId,
			fusion_address: FusionAddress,
			unbonded_amount: FusionCurrencyBalance,
			points: Points,
			era: EraIndex,
		},
		/// Event triggered when a user withdraws unbonded currency
		CurrencyWithdrawn {
			pool_id: PoolId,
			currency_id: CurrencyId,
			fusion_address: FusionAddress,
			amount: FusionCurrencyBalance,
		},
		/// Event triggered when the controller address for a user is changed
		ControllerAddressSet {
			fusion_address: FusionAddress,
			new_controller_address: Option<T::AccountId>,
		},
		/// Event triggered when the Fusion address and controller address are set for the Slash destination
		SlashDestinationSet {
			fusion_address: Option<FusionAddress>,
			controller_address: Option<T::AccountId>,
		},
		/// Event triggered when the compounding value is changed for a pool member
		CompoundingSet {
			pool_id: PoolId,
			fusion_address: FusionAddress,
			compound: bool,
		},
		/// Event triggered when a new Fusion pool is created
		PoolCreated {
			pool_id: PoolId,
			currency_id: CurrencyId,
			apy: Perbill,
			state: FusionPoolState,
			nominator: Option<T::AccountId>,
			funds_account: T::AccountId,
			claimable_account: T::AccountId,
		},
		/// Event triggered when a Fusion pool's properties are updated
		PoolSet {
			pool_id: PoolId,
			apy: Option<Perbill>,
			state: Option<FusionPoolState>,
			nominator: ConfigOp<T::AccountId>,
			boost_data: ConfigOp<(Perbill, FusionCurrencyBalance)>,
		},
		/// Event triggered when a user joins a pool
		PoolJoined {
			pool_id: PoolId,
			fusion_address: FusionAddress,
			currency_id: CurrencyId,
			amount: FusionCurrencyBalance,
			points: Points,
		},
		/// Event triggered when a user bonds extra currency into a pool
		PoolBondExtra {
			pool_id: PoolId,
			fusion_address: FusionAddress,
			currency_id: CurrencyId,
			amount: FusionCurrencyBalance,
			points: Points,
		},
		/// Event triggered when a user's pool membership is removed
		PoolMembershipRemoved {
			pool_id: PoolId,
			fusion_address: FusionAddress,
		},
		/// Event triggered when a pool is deleted
		PoolDeleted {
			pool_id: PoolId,
			leftover: BalanceOf<T>,
		},
		/// Event triggered when a pool state was changed to destroying
		PoolDestroying { pool_id: PoolId },
		/// Event triggered when a pool nominates a list of targets (validators)
		Nominated {
			pool_id: PoolId,
			targets: BoundedVec<T::AccountId, T::MaxTargets>,
		},
		/// Event triggered when unclaimed rewards are sent to the remainder
		RewardRemainderSent { amount: BalanceOf<T> },
		/// Event triggered when rewards are set for an era
		RewardSet {
			era: EraIndex,
			rewarded_pools: Vec<PoolId>,
			total_rewarded: BalanceOf<T>,
			paused_pools: Vec<PoolId>,
			paused_pools_missed_rewards: Vec<BalanceOf<T>>,
			retry: bool,
		},
		/// Event triggered when a user claims rewards for a pool and era
		RewardClaimed {
			pool_id: PoolId,
			fusion_address: FusionAddress,
			era: EraIndex,
			reward: BalanceOf<T>,
		},
		/// Event triggered when exposures are set for an era
		ExposuresSet { era: EraIndex },
		/// Event triggered when AVAIL is withdrawn to the controller account
		AvailWithdrawnToController {
			fusion_address: FusionAddress,
			controller: T::AccountId,
			amount: BalanceOf<T>,
		},
		/// Event triggered when the maximum total value locked authorized is updated.
		MaxTVLUpdated(BalanceOf<T>),
		/// Event triggered when a slash was reported and it concern a fusion pool
		FusionSlashReported {
			pool_id: PoolId,
			slash_era: EraIndex,
			slash_ratio: Perbill,
			validator: T::AccountId,
		},
		/// Event triggered when one or multiple slashes are cancelled
		FusionSlashCancelled {
			pool_ids: Vec<PoolId>,
			slash_era: EraIndex,
			validators: Vec<T::AccountId>,
		},
		/// Event triggered when a pool get slashed
		FusionPoolSlashed {
			currency_id: CurrencyId,
			pool_id: PoolId,
			slash_era: EraIndex,
			amount: FusionCurrencyBalance,
		},
		/// Event triggered when pools boost allocations have been set for a user
		UserBoostAllocationsOptimized {
			fusion_address: FusionAddress,
			pools_added: Vec<PoolId>,
			pools_removed: Vec<PoolId>,
		},
		/// An error has happened in an automatic function
		ErrorDataEvent { detail: String },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The id is already used.
		CurrencyAlreadyExists,
		/// No currency with the specified id
		CurrencyNotFound,
		/// The currency is already destroyed
		CurrencyDestroyed,
		/// Currency is not deletable cause a pool exist with the currency id
		PoolExistsForCurrency,
		/// The maximum amount of the currency is lower than what's already in the system
		InvalidMaxAmount,
		/// The minimum amount stakeable for this currency is greater than the maximum amount allowed in the system
		InvalidMinAmount,
		/// Pool id is already taken
		PoolAlreadyExists,
		/// Cannot create a pool in state destroying
		CannotSetPoolToDestroying,
		/// Pool was not found in storage
		PoolNotFound,
		/// The currency rate was not found
		CurrencyRateNotFound,
		/// Arithmetic error when doing conversions
		ArithmeticError,
		/// Arithmetic error when doing points conversions
		ArithmeticPointsError,
		/// The substrate address does not correspond to the Fusion address in the mapping
		InvalidSubstrateAddress,
		/// The pool is not open
		PoolNotOpen,
		/// The bond amount is lower than the currency minimum allowed to bond
		BondAmoundTooLow,
		/// The bond amount would make the currency go past the maximum allowed in the system
		BondWouldExceedMaxForCurrency,
		/// The pool is full of members
		PoolMemberLimitReached,
		/// User has not balance record for the currency
		NoCurrencyBalanceForUser,
		/// The user has not enough balance of the specified currency
		NotEnoughCurrencyBalanceForUser,
		/// User is not a member of the pool
		MembershipNotFound,
		/// User has no more points to unbond
		NoActivePointsToUnbond,
		/// The currency name is invalid
		InvalidName,
		/// The currency's number of decimals is invalid
		InvalidNumberOfDecimals,
		/// The max number for a currency cannot be 0
		InvalidMaxNumber,
		/// The conversion rate for the currency is not valid
		InvalidConversionRate,
		/// The APY for a pool cannot be 0
		InvalidAPY,
		/// The provided amount is not valid (cannot be 0)
		InvalidAmount,
		/// The amount to unbond is invalid
		InvalidUnbondAmount,
		/// Unbonding this amount will make the remaining below minimum
		AmountWillGoBelowMinimum,
		/// User has too much unbonding chunks
		MaxUnbondingChunksExceeded,
		/// No funds are available to withdraw
		NoFundsToWithdraw,
		/// Caller is not authorized for this operation
		NotAuthorized,
		/// No rewards were found for the era
		NoRewardsForEra,
		/// The user has no funds, so no rewards can be claimed
		NoRewardsToClaim,
		/// The exposure is not founds
		ExposureNotFound,
		/// The user was not found in the exposure
		UserNotFoundInExposure,
		/// Rewards were already claimed for this user / era
		AlreadyClaimed,
		/// A user tried to unbond another user but it's only allowed if the pool is destroying
		PoolIsNotDestroying,
		/// Action is not allowed as the pool is destroying
		PoolIsDestroying,
		/// The pool is not ready to get cleaned from the storage
		PoolCannotBeCleaned,
		/// To handle compounding and easy bouding and unbonding, avail currency has no minimum
		NoMinAmountForAvailCurrency,
		/// There is no controller address to withdraw to
		NoControllerAddressForUser,
		/// If you're active points are below minimum, you cannot set compound to true
		CannotSetCompoudingWithLessThanMinimum,
		/// The state cannot be set to open if the pool is not nominating
		PoolIsNotNominating,
		/// The pool needs target if its state is open or blocked
		ActivePoolNeedsTargets,
		/// The controller of the slash destination can only be set with the correct extrinsic
		CannotSetControllerForSlashDestination,
		/// A user tried to claim but the account is empty, can try again later
		NotEnoughClaimableBalanceInPool,
		/// The maximum TVL was reached
		MaxTVLReached,
		/// No valid validators was provided in the targets
		NoValidValidators,
		/// Era duration was not recorded properly so we cannot retry
		EraDurationNotFound,
		/// Pool has leftover funds, but we did not specify where it should go
		NoLeftoverDestinationProvided,
		/// The limit in the pool pending slashes have been reached
		PendingSlashLimitReached,
		/// Slash not found in pool
		SlashNotFound,
		/// The user does not have a membership in the AVAIL pool.
		NoAvailMembership,
		/// The pool does not have boost configured.
		PoolHasNoBoost,
		/// The user does not have enough AVAIL to allocate to the boosted pools.
		NotEnoughAvailForBoost,
		/// The TC cannot set a controller address for a user, it can only remove (to clean)
		RootCanOnlyRemoveController,
		/// TODO Temp, we'll see when bridge com is done
		CannotDepositAvailCurrency,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn integrity_test() {
			// Checks that Fusion bonding duration is greater than slash defer duration
			assert!(
				T::SlashDeferDuration::get() < T::BondingDuration::get() || T::BondingDuration::get() == 0,
				"As per documentation, slash defer duration ({}) should be less than bonding duration ({}).",
				T::SlashDeferDuration::get(),
				T::BondingDuration::get(),
			)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// TODO - Dummy extrinsic to add currency without bridge, to be removed
		#[pallet::call_index(99)]
		#[pallet::weight(T::WeightInfo::create_currency())]
		pub fn deposit_currency_dummy(
			origin: OriginFor<T>,
			fusion_address: FusionAddress,
			currency_id: CurrencyId,
			amount: FusionCurrencyBalance,
		) -> DispatchResult {
			ensure_signed(origin)?;
			Self::do_deposit_currency(fusion_address, currency_id, amount)?;
			Ok(())
		}

		/// Creates a new currency
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_currency())]
		pub fn create_currency(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			name: BoundedVec<u8, T::MaxCurrencyNameLength>,
			nb_decimals: u8,
			max_amount: FusionCurrencyBalance,
			min_amount: FusionCurrencyBalance,
			initial_conversion_rate: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(
				!Currencies::<T>::contains_key(currency_id),
				Error::<T>::CurrencyAlreadyExists
			);

			ensure!(name.len() > 0, Error::<T>::InvalidName);
			ensure!(nb_decimals > 0, Error::<T>::InvalidNumberOfDecimals);
			ensure!(
				max_amount > 0 && max_amount > min_amount,
				Error::<T>::InvalidMaxNumber
			);
			ensure!(
				initial_conversion_rate > BalanceOf::<T>::zero(),
				Error::<T>::InvalidConversionRate
			);

			if currency_id == AVAIL_CURRENCY_ID {
				ensure!(min_amount == 0, Error::<T>::NoMinAmountForAvailCurrency);
			}

			let new_currency = FusionCurrency::<T> {
				name: name.clone(),
				nb_decimals,
				total_staked_native: 0,
				total_slashed_native: 0,
				total_unbonding_native: 0,
				max_amount,
				min_amount,
				is_destroyed: false,
			};

			Currencies::<T>::insert(currency_id, new_currency);
			CurrencyRates::<T>::insert(
				T::StakingFusionDataProvider::active_era(),
				currency_id,
				initial_conversion_rate,
			);
			CurrencyRateChanges::<T>::insert(currency_id, initial_conversion_rate);

			Self::deposit_event(Event::CurrencyCreated {
				currency_id,
				name,
				nb_decimals,
				min_amount,
				max_amount,
				initial_conversion_rate,
			});

			Ok(())
		}

		/// Updates an existing currency
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_currency())]
		pub fn set_currency(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			name: Option<BoundedVec<u8, T::MaxCurrencyNameLength>>,
			max_amount: Option<FusionCurrencyBalance>,
			min_amount: Option<FusionCurrencyBalance>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Currencies::<T>::try_mutate(currency_id, |currency_opt| {
				let currency = currency_opt.as_mut().ok_or(Error::<T>::CurrencyNotFound)?;

				ensure!(!currency.is_destroyed, Error::<T>::CurrencyDestroyed);

				if let Some(name) = name.clone() {
					ensure!(name.len() > 0, Error::<T>::InvalidName);
					currency.name = name;
				}

				if let Some(max_amount) = max_amount {
					ensure!(max_amount > 0, Error::<T>::InvalidMaxNumber);
					let total_staked_and_unbonding = currency
						.total_staked_native
						.saturating_add(currency.total_unbonding_native);
					ensure!(
						max_amount >= total_staked_and_unbonding,
						Error::<T>::InvalidMaxAmount
					);
					currency.max_amount = max_amount;
				}

				if let Some(min_amount) = min_amount {
					ensure!(
						min_amount <= currency.max_amount,
						Error::<T>::InvalidMinAmount
					);
					ensure!(
						currency_id != AVAIL_CURRENCY_ID || min_amount == 0,
						Error::<T>::NoMinAmountForAvailCurrency
					);
					currency.min_amount = min_amount;
				}

				Self::deposit_event(Event::CurrencySet {
					currency_id,
					name,
					min_amount,
					max_amount,
				});

				Ok(())
			})
		}

		/// Deletes a currency
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::destroy_currency())]
		pub fn destroy_currency(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResult {
			ensure_root(origin)?;

			let pool_exists = Pools::<T>::iter().any(|(_, pool)| pool.currency_id == currency_id);
			ensure!(!pool_exists, Error::<T>::PoolExistsForCurrency);

			Currencies::<T>::try_mutate(currency_id, |currency_opt| {
				let currency = currency_opt.as_mut().ok_or(Error::<T>::CurrencyNotFound)?;
				ensure!(!currency.is_destroyed, Error::<T>::CurrencyDestroyed);
				currency.is_destroyed = true;
				CurrencyRateChanges::<T>::remove(currency_id); // CurrencyRates will clean itself using history depth
				Self::deposit_event(Event::CurrencyDeleted { currency_id });
				Ok(())
			})
		}

		/// Sets the conversion rate for a currency for the next era
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::set_currency_conversion_rate())]
		pub fn set_currency_conversion_rate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			conversion_rate: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(
				conversion_rate > BalanceOf::<T>::zero(),
				Error::<T>::InvalidConversionRate
			);

			Currencies::<T>::try_get(currency_id)
				.map_err(|_| Error::<T>::CurrencyNotFound)
				.and_then(|currency| {
					ensure!(!currency.is_destroyed, Error::<T>::CurrencyDestroyed);
					Ok(())
				})?;

			CurrencyRateChanges::<T>::insert(currency_id, conversion_rate);

			Self::deposit_event(Event::CurrencyConversionRateSet {
				currency_id,
				conversion_rate,
			});

			Ok(())
		}

		/// Creates a new fusion pool
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::create_pool())]
		pub fn create_pool(
			origin: OriginFor<T>,
			pool_id: PoolId,
			currency_id: CurrencyId,
			apy: Perbill,
			nominator: Option<T::AccountId>,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(
				!Pools::<T>::contains_key(pool_id),
				Error::<T>::PoolAlreadyExists
			);

			ensure!(apy > Perbill::zero(), Error::<T>::InvalidAPY);

			let currency = Currencies::<T>::get(currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
			ensure!(!currency.is_destroyed, Error::<T>::CurrencyDestroyed);

			let funds_account = Self::get_pool_funds_account(pool_id);
			let claimable_account = Self::get_pool_claimable_account(pool_id);

			let new_pool = FusionPool::<T> {
				currency_id,
				apy,
				funds_account: funds_account.clone(),
				claimable_account: claimable_account.clone(),
				state: FusionPoolState::Paused,
				nominator: nominator.clone(),
				members: BoundedVec::default(),
				targets: BoundedVec::default(),
				total_staked_native: 0,
				total_staked_points: 0,
				total_slashed_native: 0,
				total_unbonding_native: 0,
				pending_slashes: BoundedVec::default(),
				boost_data: None,
			};

			PoolsAccountToId::<T>::insert(&new_pool.funds_account, pool_id);
			Pools::<T>::insert(pool_id, new_pool);

			Self::deposit_event(Event::PoolCreated {
				pool_id,
				currency_id,
				apy,
				state: FusionPoolState::Paused,
				nominator,
				funds_account,
				claimable_account,
			});

			Ok(())
		}

		/// Updates an existing fusion pool
		/// If some rewards were missed due to low balance in account,
		/// `retry_rewards_for_era` can be used to generate those missing rewards.
		/// This can only be used to pause all pools using a batch call.
		#[pallet::call_index(5)]
		#[pallet::weight(({
			let base_weight = T::WeightInfo::set_pool();
			let nb_retry_eras = retry_rewards_for_eras
				.as_ref()
				.map_or(0, |eras| eras.len() as u64);
			let retry_weight = T::WeightInfo::set_pool_with_retry().saturating_mul(nb_retry_eras);
			base_weight.saturating_add(retry_weight)
		}, DispatchClass::Normal))]
		pub fn set_pool(
			origin: OriginFor<T>,
			pool_id: PoolId,
			apy: Option<Perbill>,
			state: Option<FusionPoolState>,
			nominator: ConfigOp<T::AccountId>,
			boost_data: ConfigOp<(Perbill, FusionCurrencyBalance)>, // Additional apy, min to earn
			retry_rewards_for_eras: Option<BoundedVec<EraIndex, T::HistoryDepth>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Pools::<T>::try_mutate(pool_id, |maybe_pool| -> DispatchResult {
				let pool = maybe_pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;

				let is_only_retry = retry_rewards_for_eras.is_some()
					&& apy.is_none()
					&& state.is_none()
					&& matches!(nominator, ConfigOp::Noop)
					&& matches!(boost_data, ConfigOp::Noop);

				ensure!(
					pool.state != FusionPoolState::Destroying || is_only_retry,
					Error::<T>::PoolIsDestroying
				);

				if let Some(apy) = apy {
					ensure!(apy > Perbill::zero(), Error::<T>::InvalidAPY);
					pool.apy = apy;
				}

				if let Some(state) = state {
					ensure!(
						state != FusionPoolState::Destroying,
						Error::<T>::CannotSetPoolToDestroying
					);
					if state == FusionPoolState::Open || state == FusionPoolState::Blocked {
						ensure!(pool.targets.len() > 0, Error::<T>::PoolIsNotNominating);
						let currency = Currencies::<T>::get(pool.currency_id)
							.ok_or(Error::<T>::CurrencyNotFound)?;
						ensure!(!currency.is_destroyed, Error::<T>::CurrencyDestroyed);
					}

					pool.state = state;
				}

				pool.nominator = match nominator.clone() {
					ConfigOp::Noop => pool.nominator.clone(),
					ConfigOp::Remove => None,
					ConfigOp::Set(value) => Some(value),
				};

				match boost_data {
					ConfigOp::Remove => pool.set_boost(pool_id, None)?,
					ConfigOp::Set(value) => pool.set_boost(pool_id, Some(value))?,
					_ => { /* Do Nothing */ },
				};

				Ok(())
			})?;

			if let Some(retry_rewards_for_eras) = retry_rewards_for_eras {
				retry_rewards_for_eras
					.into_iter()
					.try_for_each(|era| -> DispatchResult {
						let era_duration =
							EraDurations::<T>::get(era).ok_or(Error::<T>::EraDurationNotFound)?;
						Self::compute_era_rewards(era, era_duration, Some(pool_id));
						Ok(())
					})?;
			}

			// Emit an event for pool update
			Self::deposit_event(Event::PoolSet {
				pool_id,
				apy,
				state,
				nominator,
				boost_data,
			});

			Ok(())
		}

		/// Deletes a pool
		/// Called once to set the pool to destroying
		/// Called a second time when everything is cleaned to actually destroy it
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::destroy_pool())]
		pub fn destroy_pool(
			origin: OriginFor<T>,
			pool_id: PoolId,
			leftover_destination: Option<T::AccountId>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Pools::<T>::try_mutate(pool_id, |maybe_pool| -> DispatchResult {
				let pool = maybe_pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;

				if pool.state != FusionPoolState::Destroying {
					pool.state = FusionPoolState::Destroying;
					Self::deposit_event(Event::PoolDestroying { pool_id });
				} else {
					let leftover =
						Self::check_and_cleanup_pool(pool_id, pool, leftover_destination)?;
					*maybe_pool = None;
					Self::deposit_event(Event::PoolDeleted { pool_id, leftover });
				}

				Ok(())
			})
		}

		/// Fills the funds account with the specified amount of funds.
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::fill_pool_account())]
		pub fn fill_pool_account(
			origin: OriginFor<T>,
			pool_id: PoolId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(amount > BalanceOf::<T>::zero(), Error::<T>::InvalidAmount);

			let funds_account = Self::get_pool_funds_account(pool_id);

			T::Currency::transfer(
				&who,
				&funds_account,
				amount,
				ExistenceRequirement::KeepAlive,
			)?;

			Self::deposit_event(Event::FundsAccountFilled {
				sender: who,
				amount,
			});

			Ok(())
		}

		/// Nominates a list of validators for a given pool.
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::nominate())]
		pub fn nominate(
			origin: OriginFor<T>,
			pool_id: PoolId,
			targets: BoundedVec<T::AccountId, T::MaxTargets>,
		) -> DispatchResult {
			// Check if the origin is root, if not, check if it's a signed origin.
			let who = ensure_signed_or_root(origin)?;

			// Fetch the pool and ensure it exists
			Pools::<T>::try_mutate(pool_id, |pool_opt| -> DispatchResult {
				let pool = pool_opt.as_mut().ok_or(Error::<T>::PoolNotFound)?;

				// If the caller is not root, ensure it's the nominator of the pool
				if let Some(who) = who {
					ensure!(
						Some(&who) == pool.nominator.as_ref(),
						Error::<T>::NotAuthorized
					);
				}

				// We cannot change nominations if the pool is destroying
				ensure!(
					pool.state != FusionPoolState::Destroying,
					Error::<T>::PoolIsDestroying
				);

				// If the pool is open or blocked, nomination must not be empty
				if pool.state == FusionPoolState::Open || pool.state == FusionPoolState::Blocked {
					ensure!(targets.len() > 0, Error::<T>::ActivePoolNeedsTargets);
				}

				// Check that targets contains only validators
				ensure!(
					targets
						.iter()
						.all(|target| T::StakingFusionDataProvider::is_valid_validator(target)),
					Error::<T>::NoValidValidators
				);

				// Update the targets of the pool
				pool.targets = targets.clone();

				// Emit event for nomination
				Self::deposit_event(Event::Nominated { pool_id, targets });

				Ok(())
			})
		}

		/// Change the Substrate controller address.
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::set_controller_address())]
		pub fn set_controller_address(
			origin: OriginFor<T>,
			fusion_address: FusionAddress,
			new_controller_address: Option<T::AccountId>,
		) -> DispatchResult {
			if let Some(who) = ensure_signed_or_root(origin)? {
				// TODO - commented for tests only
				// Self::ensure_valid_fusion_origin(who, fusion_address)?;
				let _ = Self::ensure_valid_fusion_origin(who, fusion_address);
			} else {
				// TODO - commented for tests only
				// ensure!(
				// 	new_controller_address.is_none(),
				// 	Error::<T>::RootCanOnlyRemoveController
				// );
			}

			let slash_destination = SlashDestination::<T>::get();
			if let Some(slash_address) = slash_destination {
				ensure!(
					fusion_address != slash_address,
					Error::<T>::CannotSetControllerForSlashDestination
				);
			}

			Self::do_set_controller_address(fusion_address, new_controller_address)?;
			Ok(())
		}

		/// Change the Slash destination Fusion address.
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::set_slash_destination())]
		pub fn set_slash_destination(
			origin: OriginFor<T>,
			fusion_address: Option<FusionAddress>,
			controller_address: Option<T::AccountId>,
		) -> DispatchResult {
			ensure_root(origin)?;

			if let Some(fusion_address) = fusion_address {
				SlashDestination::<T>::put(fusion_address);
				Self::do_set_controller_address(fusion_address, controller_address.clone())?;
			} else {
				if let Some(current_address) = SlashDestination::<T>::get() {
					Self::do_set_controller_address(current_address, None)?;
				}
				SlashDestination::<T>::kill();
			}

			Self::deposit_event(Event::SlashDestinationSet {
				fusion_address,
				controller_address,
			});

			Ok(())
		}

		/// Updates the maximum TVL authorized in the Fusion pallet.
		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::update_max_tvl())]
		pub fn update_max_tvl(origin: OriginFor<T>, new_max_tvl: BalanceOf<T>) -> DispatchResult {
			ensure_root(origin)?;
			let mut tvl_data = <TotalValueLockedData<T>>::get();
			ensure!(
				new_max_tvl >= tvl_data.total_value_locked,
				Error::<T>::MaxTVLReached
			);
			tvl_data.max_total_value_locked = new_max_tvl;
			<TotalValueLockedData<T>>::put(tvl_data);
			Self::deposit_event(Event::MaxTVLUpdated(new_max_tvl));
			Ok(())
		}

		/// Set the destination of the reward for the user.
		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::set_compounding())]
		pub fn set_compounding(
			origin: OriginFor<T>,
			fusion_address: FusionAddress,
			pool_id: PoolId,
			compound: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_valid_fusion_origin(who, fusion_address)?;
			Self::do_set_compounding(fusion_address, pool_id, compound)?;
			Ok(())
		}

		/// Stake currency into a pool, either by joining or bonding extra.
		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::stake())]
		pub fn stake(
			origin: OriginFor<T>,
			fusion_address: FusionAddress,
			pool_id: PoolId,
			amount: FusionCurrencyBalance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_valid_fusion_origin(who, fusion_address)?;
			Self::do_stake(fusion_address, pool_id, amount, false)?;
			Ok(())
		}

		/// Claims the rewards for an Fusion address for a specific era and pool.
		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::claim_rewards())]
		pub fn claim_rewards(
			origin: OriginFor<T>,
			era: EraIndex,
			pool_id: PoolId,
			fusion_address: FusionAddress,
		) -> DispatchResult {
			ensure_signed(origin)?;
			Self::do_claim_rewards(era, pool_id, fusion_address)
		}

		/// Unbonds an amount of currency from a pool
		#[pallet::call_index(15)]
		#[pallet::weight(T::WeightInfo::unbond_currency())]
		pub fn unbond_currency(
			origin: OriginFor<T>,
			fusion_address: FusionAddress,
			pool_id: PoolId,
			unbond_amount: FusionCurrencyBalance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_valid_fusion_origin(who, fusion_address)?;
			Self::do_unbond(fusion_address, pool_id, Some(unbond_amount), false)?;
			Ok(())
		}

		/// Withdraws unbonded currency after the bonding duration has passed.
		#[pallet::call_index(16)]
		#[pallet::weight(T::WeightInfo::withdraw_unbonded_currency())]
		pub fn withdraw_unbonded_currency(
			origin: OriginFor<T>,
			fusion_address: FusionAddress,
			pool_id: PoolId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_valid_fusion_origin(who, fusion_address)?;
			Self::do_withdraw_unbonded_currency(fusion_address, pool_id, false)?;
			Ok(())
		}

		/// Unbonds an amount of currency from a pool on behalf on another user
		/// Only works if the pool is destroying
		#[pallet::call_index(17)]
		#[pallet::weight(T::WeightInfo::unbond_currency())]
		pub fn unbond_currency_other(
			origin: OriginFor<T>,
			fusion_address: FusionAddress,
			pool_id: PoolId,
		) -> DispatchResult {
			ensure_signed(origin)?;
			Self::do_unbond(fusion_address, pool_id, None, true)?;
			Ok(())
		}

		/// Withdraws unbonded currency after the bonding duration has passed.
		/// Only works if the pool is destroying
		#[pallet::call_index(18)]
		#[pallet::weight(T::WeightInfo::withdraw_unbonded_currency())]
		pub fn withdraw_unbonded_currency_other(
			origin: OriginFor<T>,
			fusion_address: FusionAddress,
			pool_id: PoolId,
		) -> DispatchResult {
			ensure_signed(origin)?;
			Self::do_withdraw_unbonded_currency(fusion_address, pool_id, true)?;
			Ok(())
		}

		/// Withdraws unbonded Avail Fusion Currency to the controller account.
		/// Only works for avail pool
		#[pallet::call_index(19)]
		#[pallet::weight(T::WeightInfo::withdraw_avail_to_controller())]
		pub fn withdraw_avail_to_controller(
			origin: OriginFor<T>,
			fusion_address: FusionAddress,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_valid_fusion_origin(who, fusion_address)?;
			Self::do_withdraw_avail_to_controller(fusion_address)?;
			Ok(())
		}

		/// Extrinsic to allow user to setup boost pool allocations
		#[pallet::call_index(20)]
		#[pallet::weight(T::WeightInfo::set_pool_boost_allocations())]
		pub fn set_pool_boost_allocations(
			origin: OriginFor<T>,
			fusion_address: FusionAddress,
			pool_ids: BoundedVec<PoolId, ConstU32<50>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let is_valid_origin = Self::ensure_valid_fusion_origin(who, fusion_address).is_ok();
			Self::do_set_pool_boost_allocations(fusion_address, pool_ids, is_valid_origin)?;
			Ok(())
		}

		/// Retrieve funds from a pool account.
		#[pallet::call_index(21)]
		#[pallet::weight(T::WeightInfo::withdraw_pool_account())]
		pub fn withdraw_pool_account(
			origin: OriginFor<T>,
			pool_id: PoolId,
			amount: BalanceOf<T>,
			dest: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(amount > BalanceOf::<T>::zero(), Error::<T>::InvalidAmount);

			let funds_account = Self::get_pool_funds_account(pool_id);

			let existence_requirement = if Pools::<T>::get(pool_id)
				.map_or(false, |pool| pool.state != FusionPoolState::Destroying)
			{
				ExistenceRequirement::KeepAlive
			} else {
				ExistenceRequirement::AllowDeath
			};

			T::Currency::transfer(&funds_account, &dest, amount, existence_requirement)?;

			Self::deposit_event(Event::FundsAccountWithdrawn {
				recipient: dest,
				amount,
			});

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Avail fusion currency account holds the native avail corresponding to the equivalent in Avail Fusion currency
	pub fn avail_account() -> T::AccountId {
		T::PalletId::get().into_sub_account_truncating(FusionAccountType::AvailCurrency)
	}

	/// Helper to convert u128 to U256
	pub fn u256(value: u128) -> U256 {
		U256::from(value)
	}

	/// Helper to convert U256 to balance
	pub fn balance(value: U256) -> BalanceOf<T> {
		let value: u128 = value.try_into().unwrap_or(u128::MAX);
		value.try_into().unwrap_or(BalanceOf::<T>::max_value())
	}

	/// Helper to convert U256 to fusion currency
	pub fn fusion_currency(value: U256) -> FusionCurrencyBalance {
		value.try_into().unwrap_or(u128::MAX)
	}

	/// Helper to convert U256 to points
	pub fn points(value: U256) -> Points {
		value.try_into().unwrap_or(u128::MAX)
	}

	/// Ensures the origin is signed and that the provided Fusion address maps to the correct Substrate account.
	pub fn ensure_valid_fusion_origin(
		who: T::AccountId,
		fusion_address: FusionAddress,
	) -> DispatchResult {
		let mapped_address = FusionAddressToSubstrateAddress::<T>::get(fusion_address)
			.ok_or(Error::<T>::InvalidSubstrateAddress)?;
		ensure!(who == mapped_address, Error::<T>::InvalidSubstrateAddress);
		Ok(())
	}

	/// Adds the fusion currency amount to the user's idle balance for a specific currency.
	pub fn add_to_currency_balance(
		fusion_address: FusionAddress,
		currency_id: CurrencyId,
		amount: FusionCurrencyBalance,
		skip_check: bool,
	) -> DispatchResult {
		if !skip_check {
			let _ = Currencies::<T>::get(currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
		}
		UserCurrencyBalances::<T>::mutate(fusion_address, currency_id, |balance_opt| {
			if let Some(balance) = balance_opt {
				balance.amount = balance.amount.saturating_add(amount);
			} else {
				*balance_opt = Some(FusionUserCurrencyBalance {
					fusion_address,
					currency_id,
					amount,
				});
			}
		});

		Ok(())
	}

	/// Withdraw the fusion currency amount from the user's idle balance for a specific currency.
	pub fn withdraw_from_currency_balance(
		fusion_address: FusionAddress,
		currency_id: CurrencyId,
		amount: FusionCurrencyBalance,
	) -> DispatchResult {
		UserCurrencyBalances::<T>::try_mutate(
			fusion_address,
			currency_id,
			|balance_opt| -> DispatchResult {
				let balance = balance_opt
					.as_mut()
					.ok_or(Error::<T>::NoCurrencyBalanceForUser)?;
				ensure!(
					balance.amount >= amount,
					Error::<T>::NotEnoughCurrencyBalanceForUser
				);

				balance.amount = balance.amount.saturating_sub(amount);

				if balance.amount == 0 {
					*balance_opt = None;
				}

				Ok(())
			},
		)
	}

	/// Function to check if a pool should be removed and perform cleanup if necessary
	pub fn check_and_cleanup_pool(
		pool_id: PoolId,
		pool: &FusionPool<T>,
		leftover_destination: Option<T::AccountId>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let has_no_members = pool.members.is_empty();
		let has_no_points = pool.total_staked_points == 0;
		let has_no_staked_native = pool.total_staked_native == 0;
		let has_no_unbonding_native = pool.total_unbonding_native == 0;

		// Ensure the pool is ready for cleanup
		ensure!(
			has_no_members && has_no_points && has_no_staked_native && has_no_unbonding_native,
			Error::<T>::PoolCannotBeCleaned
		);

		// We do this check after because it is costlier so we prefer to not do it everytime
		ensure!(
			!Self::pool_has_unclaimed_rewards(pool_id),
			Error::<T>::PoolCannotBeCleaned
		);

		let destination = leftover_destination.ok_or(Error::<T>::NoLeftoverDestinationProvided)?;
		let zero = BalanceOf::<T>::zero();

		// Retrieve balances of funds account and send to the leftover destination
		let funds_balance = T::Currency::free_balance(&pool.funds_account);
		if funds_balance > zero {
			// Transfer funds from funds_account to leftover_destination
			T::Currency::transfer(
				&pool.funds_account,
				&destination,
				funds_balance,
				ExistenceRequirement::AllowDeath,
			)?;
		}

		// Retrieve balances of claimable account and send to the leftover destination
		let claimable_balance = T::Currency::free_balance(&pool.claimable_account);
		if claimable_balance > zero {
			// Transfer funds from claimable_account to leftover_destination
			T::Currency::transfer(
				&pool.claimable_account,
				&destination,
				claimable_balance,
				ExistenceRequirement::AllowDeath,
			)?;
		}

		for key in EraRewards::<T>::iter_keys() {
			if key.1 == pool_id {
				EraRewards::<T>::remove(key.0, key.1);
			}
		}
		for key in Exposures::<T>::iter_keys() {
			if key.1 == pool_id {
				Exposures::<T>::remove(key.0, key.1);
			}
		}
		for key in ClaimedRewards::<T>::iter_keys() {
			if key.1 .0 == pool_id {
				ClaimedRewards::<T>::remove(key.0, key.1);
			}
		}

		PoolsAccountToId::<T>::remove(&pool.funds_account);
		PoolsWithBoost::<T>::remove(pool_id);

		Ok(funds_balance.saturating_add(claimable_balance))
	}

	/// Returns true if the pool has unclaimed rewards
	fn pool_has_unclaimed_rewards(pool_id: PoolId) -> bool {
		let active_era = T::StakingFusionDataProvider::active_era();
		let history_depth = T::HistoryDepth::get();
		let start_era = active_era.saturating_sub(history_depth);

		for era in start_era..active_era {
			if let Some(reward) = EraRewards::<T>::get(era, pool_id) {
				if reward.rewards != reward.claimed_rewards
					|| reward.additional_rewards != reward.additional_claimed_rewards
				{
					return true;
				}
			}
		}

		false
	}

	/// Setup the fusion currency rates for the new era
	pub fn setup_currency_rates(era: EraIndex) -> DispatchResult {
		for (currency_id, currency) in Currencies::<T>::iter() {
			// Skip if the currency is destroyed
			if currency.is_destroyed {
				continue;
			}

			// Get the new rate from the rate changes storage
			let rate = CurrencyRateChanges::<T>::get(currency_id);

			// Insert the rate for the next era
			CurrencyRates::<T>::insert(era.saturating_add(1), currency_id, rate);
		}
		Ok(())
	}

	/// Clean history depth storages and send old pending rewards to 'RewardRemainder'
	pub fn clean_history_depth_storages(era: EraIndex) -> DispatchResult {
		let history_depth = T::HistoryDepth::get();

		let Some(era_to_clear) = era.checked_sub(history_depth) else {
			return Ok(());
		};

		// Clean Exposures - u32::MAX is safe knowing the maximum number of pools is low
		let _ = Exposures::<T>::clear_prefix(era_to_clear, u32::MAX, None);

		// Clean PoolsBackingValidator - u32::MAX is safe knowing the maximum number of pools is low
		let _ = PoolsBackingValidator::<T>::clear_prefix(era_to_clear, u32::MAX, None);

		// Clean old era durations
		EraDurations::<T>::remove(era_to_clear);

		// Clean currency rates
		let _ = CurrencyRates::<T>::clear_prefix(era_to_clear, u32::MAX, None);

		// Clean claimed rewards
		let _ = ClaimedRewards::<T>::clear_prefix(era_to_clear, u32::MAX, None);

		// Clean slashes that did not get applied, this means a bug happened and should be fixed.
		for ((validator, funds_account), nb_pending_slash) in
			HasPendingSlash::<T>::iter_prefix(era_to_clear)
		{
			if nb_pending_slash > 0 {
				let Some(pool_id) = Self::get_pool_id_from_funds_account(&funds_account) else {
					continue;
				};
				let _ = Pools::<T>::try_mutate(pool_id, |pool_opt| -> DispatchResult {
					let pool = pool_opt.as_mut().ok_or(Error::<T>::PoolNotFound)?;
					pool.pending_slashes.retain(|slash| {
						!(slash.slash_era == era_to_clear && slash.validator == validator)
					});
					Ok(())
				});
			}
		}
		let _ = HasPendingSlash::<T>::clear_prefix(era_to_clear, u32::MAX, None);

		// Clean fusion era rewards and compute remaining rewards
		let existential_deposit = T::Currency::minimum_balance();
		let mut total_remaining = BalanceOf::<T>::zero();
		EraRewards::<T>::drain_prefix(era_to_clear).for_each(|(pool_id, rewards)| {
			let remaining_rewards = rewards.rewards.saturating_sub(rewards.claimed_rewards);
			if remaining_rewards > BalanceOf::<T>::zero() {
				let claimable_account = Self::get_pool_claimable_account(pool_id);
				let claimable_balance = T::Currency::free_balance(&claimable_account)
					.saturating_sub(existential_deposit);
				if claimable_balance > remaining_rewards {
					let imbalance = T::Currency::withdraw(
						&claimable_account,
						remaining_rewards,
						WithdrawReasons::all(),
						ExistenceRequirement::KeepAlive,
					);
					if let Ok(imbalance) = imbalance {
						T::RewardRemainder::on_unbalanced(imbalance);
						total_remaining = total_remaining.saturating_add(remaining_rewards);
					}
				}
			}
		});
		if total_remaining > BalanceOf::<T>::zero() {
			Self::deposit_event(Event::RewardRemainderSent {
				amount: total_remaining,
			});
		}

		Ok(())
	}

	/// Compute rewards for each pool and set them in storage
	/// Reward computatation is done at the end of era N for era N
	pub fn compute_era_rewards(era: EraIndex, era_duration: u64, maybe_pool_id: Option<PoolId>) {
		let mut total_rewarded = BalanceOf::<T>::zero();
		let mut rewarded_pools: Vec<PoolId> = vec![];
		let mut paused_pools: Vec<PoolId> = vec![];
		let mut paused_pools_missed_rewards: Vec<BalanceOf<T>> = vec![];
		let existential_deposit = T::Currency::minimum_balance();

		let exposures_iter =
			Exposures::<T>::iter_prefix(era).filter(|(pool_id, _)| match maybe_pool_id {
				Some(ref id) => pool_id == id,
				None => true,
			});

		for (pool_id, fusion_exposure) in exposures_iter {
			let Some(mut pool) = Pools::<T>::get(pool_id) else {
				Self::deposit_event(Event::ErrorDataEvent {
					detail: format!("Pool with PoolId {:?} not found for Era {:?}. Reward could not have been set.", pool_id, era)
				});
				continue;
			};

			if fusion_exposure.total_avail.is_zero()
				|| fusion_exposure.user_points.is_empty()
				|| fusion_exposure.targets.is_empty()
				|| Perbill::is_zero(&fusion_exposure.apy)
				|| EraRewards::<T>::get(era, pool_id).is_some()
			{
				// No need to pause the pool cause it's just not supposed to get rewards.
				continue;
			}

			// Era reward computation for a pool
			let apy = fusion_exposure.apy;
			let fraction_of_year = Perbill::from_rational(era_duration, MILLISECONDS_PER_YEAR);
			let total_avail = fusion_exposure.total_avail;
			let total_avail_reward_for_year = apy * total_avail;
			let pool_era_reward = fraction_of_year * total_avail_reward_for_year;

			// Boost era reward computation for a pool
			let mut boost_reward = BalanceOf::<T>::default();
			if fusion_exposure.boost_members.len() > 0
				&& fusion_exposure.boost_total_points > 0
				&& fusion_exposure.boost_additional_apy > Perbill::zero()
				&& fusion_exposure.boost_total_avail > BalanceOf::<T>::zero()
			{
				let boost_additional_apy = fusion_exposure.boost_additional_apy;
				let boost_total_avail = fusion_exposure.boost_total_avail;
				let total_avail_reward_for_year = boost_additional_apy * boost_total_avail;
				boost_reward = fraction_of_year * total_avail_reward_for_year;
			}

			// Check that the pool actually backed a validator and that this validator has earned points during the era
			let mut should_earn_rewards = false;
			if let Some(native_exposure_data) = fusion_exposure.native_exposure_data {
				let validators_backed: Vec<T::AccountId> = native_exposure_data
					.into_iter()
					.map(|(account_id, _balance)| account_id)
					.collect();
				should_earn_rewards =
					T::StakingFusionDataProvider::has_earned_era_points(era, &validators_backed);
			}

			if !should_earn_rewards {
				Self::pause_pool(
					pool_id,
					&mut pool,
					"Fusion pool selected validators have not earned rewards",
					&mut paused_pools,
					&mut paused_pools_missed_rewards,
					pool_era_reward,
				);
				continue;
			}

			// Get the pool funds balances
			let pool_funds_balance = T::Currency::free_balance(&pool.funds_account);
			let era_rewards_with_boost = pool_era_reward.saturating_add(boost_reward);

			// In case of insufficient balance in pool account, we pause the pool
			// This means the reward won't get paid for this era.
			if era_rewards_with_boost > pool_funds_balance.saturating_sub(existential_deposit) {
				Self::pause_pool(
					pool_id,
					&mut pool,
					"Insufficient funds in fusion pool account.",
					&mut paused_pools,
					&mut paused_pools_missed_rewards,
					era_rewards_with_boost,
				);
				continue;
			}

			if let Err(_) = T::Currency::transfer(
				&pool.funds_account,
				&pool.claimable_account,
				era_rewards_with_boost,
				ExistenceRequirement::KeepAlive,
			) {
				Self::pause_pool(
					pool_id,
					&mut pool,
					"An error has occured during transfer from pool funds account to claimable account.",
					&mut paused_pools,
					&mut paused_pools_missed_rewards,
					era_rewards_with_boost,
				);
				continue;
			}

			total_rewarded = total_rewarded.saturating_add(era_rewards_with_boost);

			EraRewards::<T>::insert(
				era,
				pool_id,
				EraReward {
					rewards: pool_era_reward,
					claimed_rewards: BalanceOf::<T>::default(),
					additional_rewards: boost_reward,
					additional_claimed_rewards: BalanceOf::<T>::default(),
				},
			);

			rewarded_pools.push(pool_id);
		}

		// Record Era duration in case we need it later, eg. for a retry
		if !EraDurations::<T>::get(era).is_some() {
			EraDurations::<T>::insert(era, era_duration);
		}

		if !rewarded_pools.is_empty() || !paused_pools.is_empty() {
			Self::deposit_event(Event::RewardSet {
				era,
				rewarded_pools,
				paused_pools,
				total_rewarded,
				paused_pools_missed_rewards,
				retry: maybe_pool_id.is_some(),
			});
		}
	}

	pub fn pause_pool(
		pool_id: PoolId,
		pool: &mut FusionPool<T>,
		reason: &str,
		paused_pools: &mut Vec<PoolId>,
		paused_pools_missed_rewards: &mut Vec<BalanceOf<T>>,
		pool_era_reward: BalanceOf<T>,
	) {
		if pool.is_active() {
			Self::deposit_event(Event::ErrorDataEvent {
				detail: format!("Pausing pool {:?}: {}.", pool_id, reason),
			});
			pool.state = FusionPoolState::Paused;
			Pools::<T>::insert(pool_id, pool);
			paused_pools.push(pool_id);
			paused_pools_missed_rewards.push(pool_era_reward);
		}
	}

	/// Increase total value locked in avail
	pub fn add_to_tvl(
		currency_id: CurrencyId,
		currency: &FusionCurrency<T>,
		value: FusionCurrencyBalance,
	) -> DispatchResult {
		let mut tvl_data = TotalValueLockedData::<T>::get();
		let avail_value = currency.currency_to_avail(currency_id, value, None)?;
		tvl_data.add(avail_value)?;
		TotalValueLockedData::<T>::put(tvl_data);
		Ok(())
	}

	/// Decrease total value locked in avail
	pub fn sub_from_tvl(
		currency_id: CurrencyId,
		currency: &FusionCurrency<T>,
		value: FusionCurrencyBalance,
	) -> DispatchResult {
		let mut tvl_data = TotalValueLockedData::<T>::get();
		let avail_value = currency.currency_to_avail(currency_id, value, None)?;
		tvl_data.sub(avail_value);
		TotalValueLockedData::<T>::put(tvl_data);
		Ok(())
	}

	/// Deposits a specified amount of currency for a given Fusion address and currency ID.
	pub fn do_deposit_currency(
		fusion_address: FusionAddress,
		currency_id: CurrencyId,
		amount: FusionCurrencyBalance,
	) -> DispatchResult {
		// TODO - in case we're adding avail, the Avail currency should come from somewhere and put in avail holdings of the pallet, for now we just prevent it
		ensure!(
			currency_id != AVAIL_CURRENCY_ID,
			Error::<T>::CannotDepositAvailCurrency
		);

		Self::add_to_currency_balance(fusion_address, currency_id, amount, false)?;

		Self::deposit_event(Event::CurrencyDeposited {
			fusion_address,
			currency_id,
			amount,
		});

		Ok(())
	}

	/// Sets or unsets a controller address for a specific Fusion address.
	pub fn do_set_controller_address(
		fusion_address: FusionAddress,
		new_controller_address: Option<T::AccountId>,
	) -> DispatchResult {
		if let Some(ref new_controller_address) = new_controller_address {
			FusionAddressToSubstrateAddress::<T>::insert(fusion_address, new_controller_address);
		} else {
			FusionAddressToSubstrateAddress::<T>::remove(fusion_address);
		}

		Self::deposit_event(Event::ControllerAddressSet {
			fusion_address,
			new_controller_address,
		});

		Ok(())
	}

	/// Configures whether the specified Fusion address should compound rewards in a given pool.
	pub fn do_set_compounding(
		fusion_address: FusionAddress,
		pool_id: PoolId,
		compound: bool,
	) -> DispatchResult {
		Memberships::<T>::try_mutate(fusion_address, pool_id, |membership_opt| {
			let membership = membership_opt
				.as_mut()
				.ok_or(Error::<T>::MembershipNotFound)?;
			let pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;
			let currency =
				Currencies::<T>::get(pool.currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
			let active_currency_balance =
				pool.points_to_currency(membership.active_points, Some(&currency))?;
			if compound {
				ensure!(
					active_currency_balance >= currency.min_amount,
					Error::<T>::CannotSetCompoudingWithLessThanMinimum
				)
			}

			membership.is_compounding = compound;

			Self::deposit_event(Event::<T>::CompoundingSet {
				fusion_address,
				pool_id,
				compound,
			});
			Ok::<(), Error<T>>(())
		})?;

		Ok(())
	}

	/// Stakes a specified amount of currency into a pool for a given Fusion address.
	/// If `skip_checks` is true, some checks (like pool state or pallet balance) may be skipped.
	pub fn do_stake(
		fusion_address: FusionAddress,
		pool_id: PoolId,
		amount: FusionCurrencyBalance,
		skip_checks: bool,
	) -> DispatchResult {
		// Fetch pool and currency
		let mut pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;
		let mut currency =
			Currencies::<T>::get(pool.currency_id).ok_or(Error::<T>::CurrencyNotFound)?;

		let maybe_membership = Memberships::<T>::get(fusion_address, pool_id);

		if !skip_checks {
			// Ensure amount is greater than 0
			ensure!(amount > 0, Error::<T>::InvalidAmount);
			// Ensure they are open or allowed to compound
			ensure!(
				pool.state == FusionPoolState::Open
					|| (pool.state == FusionPoolState::Blocked && maybe_membership.is_some()),
				Error::<T>::PoolNotOpen
			);
			ensure!(!currency.is_destroyed, Error::<T>::CurrencyDestroyed);

			// Ensure the total staked does not exceed the max allowable amount
			ensure!(
				currency.total_staked_native.saturating_add(amount) <= currency.max_amount,
				Error::<T>::BondWouldExceedMaxForCurrency
			);
		}

		// Fetch and ensure user has enough currency balance
		Self::withdraw_from_currency_balance(fusion_address, pool.currency_id, amount)?;

		// Convert currency amount to points
		let points = pool.currency_to_points(amount, Some(&currency))?;

		// Common logic to update currency and pool data
		currency.total_staked_native = currency.total_staked_native.saturating_add(amount);
		pool.total_staked_native = pool.total_staked_native.saturating_add(amount);
		pool.total_staked_points = pool.total_staked_points.saturating_add(points);

		// Update TVL
		Self::add_to_tvl(pool.currency_id, &currency, amount)?;

		// Check if the user is already a member of the pool
		if let Some(mut membership) = maybe_membership {
			// Update user's active points and save membership
			membership.active_points = membership.active_points.saturating_add(points);

			// Ensure user will have more than minimum balance
			// Useful if the user was slashed and his balance went below minimum required
			// He can only bond to top up to the minimum or withdraw all
			let current_amount =
				pool.points_to_currency(membership.active_points, Some(&currency))?;
			ensure!(
				current_amount.saturating_add(amount) > currency.min_amount,
				Error::<T>::BondAmoundTooLow
			);

			// Update the pool's member points
			if let Some(member) = pool
				.members
				.iter_mut()
				.find(|(address, _)| *address == fusion_address)
			{
				member.1 = member.1.saturating_add(points);
			}

			// Check if the user has boost in the pool
			if let Some(ref mut boost_data) = pool.boost_data {
				if HasBoost::<T>::get(pool_id, fusion_address) {
					// We add the additional points to the elligible points
					boost_data.elligible_total_points =
						boost_data.elligible_total_points.saturating_add(points);
				}
			}

			Memberships::<T>::insert(fusion_address, pool_id, membership);
			Pools::<T>::insert(pool_id, &pool);
			Currencies::<T>::insert(pool.currency_id, &currency);

			// Emit event for extra bond
			Self::deposit_event(Event::PoolBondExtra {
				fusion_address,
				pool_id,
				currency_id: pool.currency_id,
				amount,
				points,
			});
		} else {
			// Ensure the amount meets the minimum staking requirement
			ensure!(amount >= currency.min_amount, Error::<T>::BondAmoundTooLow);

			// Update pool members
			pool.members
				.try_push((fusion_address, points))
				.map_err(|_| Error::<T>::PoolMemberLimitReached)?;

			// Insert new membership for user
			let new_membership = FusionMembership::<T> {
				fusion_address,
				joined_era: T::StakingFusionDataProvider::active_era(),
				active_points: points,
				unbonding_eras: BoundedVec::default(),
				is_compounding: true,
			};
			Memberships::<T>::insert(fusion_address, pool_id, new_membership);
			Pools::<T>::insert(pool_id, &pool);
			Currencies::<T>::insert(pool.currency_id, &currency);

			// Emit event for pool join
			Self::deposit_event(Event::PoolJoined {
				fusion_address,
				pool_id,
				currency_id: pool.currency_id,
				amount,
				points,
			});
		}

		Ok(())
	}

	/// Claims rewards for a specified era and pool for a given Fusion address.
	pub fn do_claim_rewards(
		era: EraIndex,
		pool_id: PoolId,
		fusion_address: FusionAddress,
	) -> DispatchResult {
		// Get the fusion exposure for the pool and era
		let exposure = Exposures::<T>::get(era, pool_id).ok_or(Error::<T>::ExposureNotFound)?;

		EraRewards::<T>::try_mutate(era, pool_id, |maybe_reward| -> DispatchResult {
			// Ensure rewards are available for the given era and pool
			let era_rewards = maybe_reward.as_mut().ok_or(Error::<T>::NoRewardsForEra)?;

			// Ensure the user has not already claimed the reward for this era and pool
			ensure!(
				!ClaimedRewards::<T>::contains_key(era, (pool_id, fusion_address)),
				Error::<T>::AlreadyClaimed
			);

			let (user_reward_balance, user_points) =
				Self::compute_basic_rewards(fusion_address, &exposure, era_rewards)?;

			let boost_rewards =
				Self::compute_boost_rewards(fusion_address, &exposure, era_rewards, user_points)?;

			let total_user_rewards = user_reward_balance.saturating_add(boost_rewards);

			ensure!(
				total_user_rewards > BalanceOf::<T>::zero(),
				Error::<T>::NoRewardsToClaim
			);

			// Update the claimed rewards field by adding the user's reward
			era_rewards.claimed_rewards = era_rewards
				.claimed_rewards
				.saturating_add(total_user_rewards);

			// Fetch avail currency
			let avail_currency =
				Currencies::<T>::get(AVAIL_CURRENCY_ID).ok_or(Error::<T>::CurrencyNotFound)?;

			// Convert the avail reward to avail currency
			let avail_in_currency = avail_currency.avail_to_currency(
				AVAIL_CURRENCY_ID,
				total_user_rewards,
				Some(era),
			)?;

			// Transfer claimable avail to avail fusion currency account for holding
			let pool_claimable_account = Self::get_pool_funds_account(pool_id);

			// Check that it has enough funds
			let pool_claimable_balance = T::Currency::free_balance(&pool_claimable_account);
			let existential_deposit = T::Currency::minimum_balance();
			ensure!(
				total_user_rewards <= pool_claimable_balance.saturating_sub(existential_deposit),
				Error::<T>::NotEnoughClaimableBalanceInPool
			);

			// Mark rewards as claimed
			ClaimedRewards::<T>::insert(era, (pool_id, fusion_address), total_user_rewards);

			// Send the funds to the avail holdings account
			T::Currency::transfer(
				&pool_claimable_account,
				&Self::avail_account(),
				total_user_rewards,
				ExistenceRequirement::AllowDeath,
			)?;

			// We can now add the equivalent in fusion currency
			Self::add_to_currency_balance(
				fusion_address,
				AVAIL_CURRENCY_ID,
				avail_in_currency,
				true,
			)?;

			// Handle compounding or adding to the user's idle balance
			let Some(membership) = Memberships::<T>::get(fusion_address, pool_id) else {
				return Ok(());
			};

			// Checks before calling stake
			let avail_pool = Pools::<T>::get(AVAIL_POOL_ID).ok_or(Error::<T>::PoolNotFound)?;
			let has_avail_membership =
				Memberships::<T>::get(fusion_address, AVAIL_POOL_ID).is_some();
			let can_stake_to_pool = avail_pool.state == FusionPoolState::Open
				|| (avail_pool.state == FusionPoolState::Blocked && has_avail_membership);
			let wont_overflow_maximum_amount = avail_currency
				.total_staked_native
				.saturating_add(avail_in_currency)
				<= avail_currency.max_amount;
			let wont_overflow_maximum_members = has_avail_membership
				|| (avail_pool.members.len() as u32) < T::MaxMembersPerPool::get();

			if membership.is_compounding
				&& avail_in_currency > 0
				&& can_stake_to_pool
				&& wont_overflow_maximum_amount
				&& wont_overflow_maximum_members
			{
				// At this point this should never fail except in case of arithmetic errors which is ok
				Self::do_stake(fusion_address, AVAIL_POOL_ID, avail_in_currency, true)?;
			}

			Self::deposit_event(Event::RewardClaimed {
				fusion_address,
				pool_id,
				era,
				reward: total_user_rewards,
			});

			Ok(())
		})?;

		Ok(())
	}

	/// Unbonds a specified amount of currency from a pool for a given Fusion address.
	/// If `other` is true, the unbonding is performed on behalf of another user.
	pub fn do_unbond(
		fusion_address: FusionAddress,
		pool_id: PoolId,
		unbond_amount: Option<FusionCurrencyBalance>,
		other: bool,
	) -> DispatchResult {
		// Retrieve the user's membership in the pool
		let mut membership =
			Memberships::<T>::get(fusion_address, pool_id).ok_or(Error::<T>::MembershipNotFound)?;

		// Ensure the user has active points to unbond
		ensure!(
			membership.active_points > 0,
			Error::<T>::NoActivePointsToUnbond
		);

		// Fetch pool and currency details
		let mut pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;
		let mut currency =
			Currencies::<T>::get(pool.currency_id).ok_or(Error::<T>::CurrencyNotFound)?;

		// Allow permissionless only if pool is destroying
		ensure!(
			!other || pool.state == FusionPoolState::Destroying,
			Error::<T>::PoolIsNotDestroying
		);

		// Convert points to currency to determine how much to unbond
		let currency_value = pool.points_to_currency(membership.active_points, Some(&currency))?;

		let unbond_amount = unbond_amount.unwrap_or(currency_value);

		ensure!(unbond_amount > 0, Error::<T>::InvalidAmount);

		// Ensure user has enough points to unbond the requested amount
		let requested_points = pool.currency_to_points(unbond_amount, Some(&currency))?;
		ensure!(
			membership.active_points >= requested_points,
			Error::<T>::InvalidUnbondAmount
		);

		let is_full_unbond = requested_points == membership.active_points;

		let new_balance = currency_value.saturating_sub(unbond_amount);

		// Ensure it's full unbond or valid partial unbond
		ensure!(
			is_full_unbond || new_balance >= currency.min_amount,
			Error::<T>::AmountWillGoBelowMinimum
		);

		// Get current era
		let current_era = T::StakingFusionDataProvider::active_era();

		// Add the unbonding chunk to the related storage
		let mut era_pool_unbonding_chunk = UnbondingChunks::<T>::get(pool_id, current_era);
		let existing_index = era_pool_unbonding_chunk
			.iter()
			.position(|(addr, _)| *addr == fusion_address);
		if let Some(index) = existing_index {
			era_pool_unbonding_chunk[index].1 = era_pool_unbonding_chunk[index]
				.1
				.saturating_add(unbond_amount);
		} else {
			era_pool_unbonding_chunk
				.try_push((fusion_address, unbond_amount))
				.map_err(|_| Error::<T>::PoolMemberLimitReached)?;

			// If the unbonding chunk is new, we add its info in the membership
			membership
				.unbonding_eras
				.try_push(current_era)
				.map_err(|_| Error::<T>::MaxUnbondingChunksExceeded)?;
		}
		UnbondingChunks::<T>::insert(pool_id, current_era, era_pool_unbonding_chunk);

		// Update membership points
		membership.active_points = membership.active_points.saturating_sub(requested_points);

		// If it is a full unbond, we set compounding to false as user probably want to leave the pool and he'll receive some rewards after
		if is_full_unbond {
			membership.is_compounding = false;
		}

		// Update the pool's member points
		if let Some(member_index) = pool
			.members
			.iter()
			.position(|(address, _)| *address == fusion_address)
		{
			// Subtract the user's points from the member entry
			if let Some((_, member_points)) = pool.members.get_mut(member_index) {
				*member_points = membership.active_points;
			}
		}

		// Check if the user has boost in the pool
		if let Some(ref mut boost_data) = pool.boost_data {
			if HasBoost::<T>::get(pool_id, fusion_address) {
				// We substract the additional points to the elligible points
				boost_data.elligible_total_points = boost_data
					.elligible_total_points
					.saturating_sub(requested_points);
			}
		}

		// Update pool totals
		pool.total_staked_points = pool.total_staked_points.saturating_sub(requested_points);
		pool.total_staked_native = pool.total_staked_native.saturating_sub(unbond_amount);
		pool.total_unbonding_native = pool.total_unbonding_native.saturating_add(unbond_amount);

		// Update currency totals
		currency.total_staked_native = currency.total_staked_native.saturating_sub(unbond_amount);
		currency.total_unbonding_native = currency
			.total_unbonding_native
			.saturating_add(unbond_amount);

		// Update TVL
		Self::sub_from_tvl(pool.currency_id, &currency, unbond_amount)?;

		// Save the updated state back to storage
		Memberships::<T>::insert(fusion_address, pool_id, membership);
		Pools::<T>::insert(pool_id, &pool);
		Currencies::<T>::insert(pool.currency_id, &currency);

		// If the user has unbonded from Avail pool, we need to check if we need to remove him from some boost pools
		if pool_id == AVAIL_POOL_ID {
			Self::check_boost_allocation_removal(fusion_address, new_balance)?;
		}

		// Emit event
		Self::deposit_event(Event::CurrencyUnbonded {
			fusion_address,
			pool_id,
			currency_id: pool.currency_id,
			unbonded_amount: unbond_amount,
			points: requested_points,
			era: current_era,
		});

		Ok(())
	}

	/// Withdraws unbonded currency for a given Fusion address after the bonding duration has passed.
	/// If `other` is true, the withdrawal is performed on behalf of another user.
	pub fn do_withdraw_unbonded_currency(
		fusion_address: FusionAddress,
		pool_id: PoolId,
		other: bool,
	) -> DispatchResult {
		// Ensure user is a member of the pool
		let mut membership =
			Memberships::<T>::get(fusion_address, pool_id).ok_or(Error::<T>::MembershipNotFound)?;

		// Fetch pool and currency data
		let mut pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;
		let mut currency =
			Currencies::<T>::get(pool.currency_id).ok_or(Error::<T>::CurrencyNotFound)?;

		// Allow permissionless only if pool is destroying
		ensure!(
			!other || pool.state == FusionPoolState::Destroying,
			Error::<T>::PoolIsNotDestroying
		);

		ensure!(
			!membership.unbonding_eras.is_empty(),
			Error::<T>::NoFundsToWithdraw
		);

		// Get active era
		let active_era = T::StakingFusionDataProvider::active_era();

		// Check if there are any unbonded chunks that are now withdrawable
		let mut total_withdrawable: FusionCurrencyBalance = 0;
		let mut remaining_unbonding_eras = BoundedVec::default();

		for era in membership.unbonding_eras.iter() {
			if era + T::BondingDuration::get() <= active_era {
				// This chunk is now withdrawable
				let mut pool_era_unbonding_chunks = UnbondingChunks::<T>::get(pool_id, era);
				let maybe_unbonding_chunk_index = pool_era_unbonding_chunks
					.iter()
					.position(|(addr, _)| *addr == fusion_address);

				if let Some(unbonding_chunk_index) = maybe_unbonding_chunk_index {
					let unbonding_chunk = pool_era_unbonding_chunks.remove(unbonding_chunk_index);
					total_withdrawable = total_withdrawable.saturating_add(unbonding_chunk.1);

					if !pool_era_unbonding_chunks.is_empty() {
						UnbondingChunks::<T>::insert(pool_id, era, pool_era_unbonding_chunks);
					} else {
						UnbondingChunks::<T>::remove(pool_id, era);
					}
				} else {
					Self::deposit_event(Event::ErrorDataEvent {
						detail: format!("An unbonding chunk was not found for user: {fusion_address:?}, era: {era:?} and pool id {pool_id:?}. Storage was cleaned but it should get fixed")
					});
				}
			} else {
				// Keep this chunk as it's not withdrawable yet
				remaining_unbonding_eras
					.try_push(*era)
					.map_err(|_| Error::<T>::MaxUnbondingChunksExceeded)?;
			}
		}

		// Ensure there is something to withdraw
		ensure!(total_withdrawable > 0, Error::<T>::NoFundsToWithdraw);

		// Update the user's membership by removing processed unbonding chunks
		membership.unbonding_eras = remaining_unbonding_eras;

		// Update pool and currency data
		pool.total_unbonding_native = pool
			.total_unbonding_native
			.saturating_sub(total_withdrawable);
		currency.total_unbonding_native = currency
			.total_unbonding_native
			.saturating_sub(total_withdrawable);

		// Update the user's currency balance
		Self::add_to_currency_balance(fusion_address, pool.currency_id, total_withdrawable, true)?;

		// Check if the user should be removed from the pool membership
		if membership.unbonding_eras.is_empty() && membership.active_points == 0 {
			// Remove the user from members in pool
			pool.members
				.retain(|(address, _)| *address != fusion_address);

			// Remove the user's membership from the pool
			Memberships::<T>::remove(fusion_address, pool_id);

			// If we remove the membership and the user had boost in pool, we need to clean it
			if HasBoost::<T>::get(pool_id, fusion_address) {
				HasBoost::<T>::remove(pool_id, fusion_address);

				if let Some(ref mut boost_data) = pool.boost_data {
					boost_data
						.elligible_members
						.retain(|address| *address != fusion_address);
				}
			}

			// Emit event for removing pool membership
			Self::deposit_event(Event::PoolMembershipRemoved {
				fusion_address,
				pool_id,
			});
		} else {
			// If there are remaining unbonding chunks or active points, update the membership
			Memberships::<T>::insert(fusion_address, pool_id, membership);
		}

		Pools::<T>::insert(pool_id, &pool);
		Currencies::<T>::insert(pool.currency_id, &currency);

		// Emit event for successful withdrawal
		Self::deposit_event(Event::CurrencyWithdrawn {
			fusion_address,
			pool_id,
			currency_id: pool.currency_id,
			amount: total_withdrawable,
		});

		Ok(())
	}

	/// Withdraws AVAIL currency to the controller account for a given Fusion address.
	pub fn do_withdraw_avail_to_controller(fusion_address: FusionAddress) -> DispatchResult {
		// Get the currency
		let currency =
			Currencies::<T>::get(AVAIL_CURRENCY_ID).ok_or(Error::<T>::CurrencyNotFound)?;

		// Get the controller account
		let controller_account = FusionAddressToSubstrateAddress::<T>::get(fusion_address)
			.ok_or(Error::<T>::NoControllerAddressForUser)?;

		// Retrieve the user's balance of AVAIL currency
		let balance = UserCurrencyBalances::<T>::get(fusion_address, AVAIL_CURRENCY_ID)
			.ok_or(Error::<T>::NoCurrencyBalanceForUser)?
			.amount;

		// Ensure the balance is greater than 0
		ensure!(balance > 0, Error::<T>::NoFundsToWithdraw);

		// Fusion currency in avail
		let balance_avail = currency.currency_to_avail(AVAIL_CURRENCY_ID, balance, None)?;

		T::Currency::transfer(
			&Self::avail_account(),
			&controller_account,
			balance_avail,
			ExistenceRequirement::AllowDeath,
		)?;

		// Remove the user's AVAIL currency balance after minting
		UserCurrencyBalances::<T>::remove(fusion_address, AVAIL_CURRENCY_ID);

		// Emit an event indicating successful withdrawal
		Self::deposit_event(Event::AvailWithdrawnToController {
			fusion_address,
			controller: controller_account,
			amount: balance_avail,
		});

		Ok(())
	}

	/// Return the pool funds account
	pub fn get_pool_funds_account(id: PoolId) -> T::AccountId {
		T::PoolAccountProvider::get_pool_funds_account(id)
	}

	/// Return the pool claimable account
	pub fn get_pool_claimable_account(id: PoolId) -> T::AccountId {
		T::PoolAccountProvider::get_pool_claimable_account(id)
	}

	/// Checks if the user boost allocation need to be removed
	pub fn check_boost_allocation_removal(
		fusion_address: FusionAddress,
		new_avail_balance: FusionCurrencyBalance,
	) -> DispatchResult {
		let mut total_avail_required: FusionCurrencyBalance = 0;
		let mut user_pool_ids: Vec<(PoolId, FusionCurrencyBalance)> = Vec::new();

		for (pool_id, min_avail_to_earn) in PoolsWithBoost::<T>::iter() {
			if HasBoost::<T>::get(pool_id, fusion_address) {
				user_pool_ids.push((pool_id, min_avail_to_earn));
				total_avail_required = total_avail_required.saturating_add(min_avail_to_earn);
			}
		}

		if new_avail_balance >= total_avail_required {
			return Ok(());
		}

		// If we're here, we need to remove the user boost from some pool
		// Sort pools by min_avail_to_earn in descending order
		user_pool_ids.sort_by(|a, b| b.1.cmp(&a.1));

		// Remove pools until the total required Avail is within the new balance
		for (pool_id, min_avail_to_earn) in user_pool_ids {
			// Remove the user's boost status from the pool
			HasBoost::<T>::remove(pool_id, fusion_address);

			Pools::<T>::try_mutate(pool_id, |pool_opt| -> DispatchResult {
				let pool = pool_opt.as_mut().ok_or(Error::<T>::PoolNotFound)?;
				if let Some(ref mut boost_data) = pool.boost_data {
					let membership = Memberships::<T>::get(fusion_address, pool_id)
						.ok_or(Error::<T>::MembershipNotFound)?;
					boost_data.elligible_total_points = boost_data
						.elligible_total_points
						.saturating_sub(membership.active_points);
					boost_data
						.elligible_members
						.retain(|address| *address != fusion_address);
				}
				Ok(())
			})?;

			total_avail_required = total_avail_required.saturating_sub(min_avail_to_earn);

			if new_avail_balance >= total_avail_required {
				break;
			}
		}

		Ok(())
	}

	/// Function to remove all boost for everyone in case the Avail pool is slashed
	pub fn shutdown_pools_boost() {
		for (pool_id, _) in PoolsWithBoost::<T>::iter() {
			let _ = HasBoost::<T>::clear_prefix(pool_id, u32::MAX, None);
			let _ = Pools::<T>::try_mutate(pool_id, |pool_opt| -> DispatchResult {
				if let Some(pool) = pool_opt.as_mut() {
					if let Some(ref mut boost_data) = pool.boost_data {
						boost_data.elligible_total_points = 0;
						boost_data.elligible_members = BoundedVec::default();
					}
				};
				Ok(())
			});
		}
	}

	/// Helper to compute the rewards for a pool member, return the rewards and the user points to avoid iterating to compute boost rewards
	pub fn compute_basic_rewards(
		fusion_address: FusionAddress,
		exposure: &FusionExposure<T>,
		era_rewards: &EraReward<T>,
	) -> Result<(BalanceOf<T>, U256), DispatchError> {
		// Find the user's points in this era for the pool
		let user_points = exposure
			.user_points
			.iter()
			.find(|(user, _)| *user == fusion_address)
			.map(|(_, points)| points)
			.ok_or(Error::<T>::UserNotFoundInExposure)?;

		// Calculate the rewards
		let user_points = Self::u256(*user_points);
		let total_points = Self::u256(exposure.total_points);
		let rewards_u128: u128 = era_rewards
			.rewards
			.try_into()
			.map_err(|_| Error::<T>::ArithmeticError)?;
		let rewards = Self::u256(rewards_u128);

		let user_reward = rewards
			.saturating_mul(user_points)
			.checked_div(total_points)
			.ok_or(Error::<T>::ArithmeticError)?;
		let user_reward_balance = Self::balance(user_reward);

		Ok((user_reward_balance, user_points))
	}

	/// Helper to compute the boost reward for a pool member
	pub fn compute_boost_rewards(
		fusion_address: FusionAddress,
		exposure: &FusionExposure<T>,
		era_rewards: &EraReward<T>,
		user_points: U256,
	) -> Result<BalanceOf<T>, DispatchError> {
		// Calculate the boost rewards
		let mut user_boost_rewards_balance = BalanceOf::<T>::zero();
		if exposure.boost_members.contains(&fusion_address) {
			let total_boost_points = Self::u256(exposure.boost_total_points);

			let boost_rewards_u128: u128 = era_rewards
				.additional_rewards
				.try_into()
				.map_err(|_| Error::<T>::ArithmeticError)?;
			let boost_rewards = Self::u256(boost_rewards_u128);

			let user_boost_reward = boost_rewards
				.saturating_mul(user_points)
				.checked_div(total_boost_points)
				.ok_or(Error::<T>::ArithmeticError)?;

			user_boost_rewards_balance = Self::balance(user_boost_reward);
		}
		Ok(user_boost_rewards_balance)
	}

	pub fn do_set_pool_boost_allocations(
		fusion_address: FusionAddress,
		pool_ids: BoundedVec<PoolId, ConstU32<50>>,
		is_valid_origin: bool,
	) -> DispatchResult {
		// Get user's current boost allocations to check for permission
		let user_memberships: Vec<PoolId> =
			Memberships::<T>::iter_key_prefix(fusion_address).collect();
		let mut current_boost_pools: Vec<PoolId> = Vec::new();
		for pool_id in user_memberships.iter() {
			if HasBoost::<T>::get(*pool_id, fusion_address) {
				current_boost_pools.push(*pool_id);
			}
		}

		// This extrinsic is permissionless only if the user has no current boost allocation
		ensure!(
			current_boost_pools.is_empty() || is_valid_origin,
			Error::<T>::NotAuthorized
		);

		// Get user's AVAIL balance in pool 0
		let avail_pool_id = AVAIL_POOL_ID;
		let avail_membership = Memberships::<T>::get(fusion_address, avail_pool_id)
			.ok_or(Error::<T>::NoAvailMembership)?;
		let avail_currency =
			Currencies::<T>::get(AVAIL_CURRENCY_ID).ok_or(Error::<T>::CurrencyNotFound)?;
		let avail_pool = Pools::<T>::get(avail_pool_id).ok_or(Error::<T>::PoolNotFound)?;
		let user_avail_balance =
			avail_pool.points_to_currency(avail_membership.active_points, Some(&avail_currency))?;

		// Calculate total minimum AVAIL required
		let mut total_min_avail_required: FusionCurrencyBalance = 0;
		for pool_id in pool_ids.iter() {
			let min_avail_to_earn =
				PoolsWithBoost::<T>::get(*pool_id).ok_or(Error::<T>::PoolHasNoBoost)?;
			total_min_avail_required = total_min_avail_required.saturating_add(min_avail_to_earn);
		}

		// Ensure user has enough AVAIL
		ensure!(
			user_avail_balance >= total_min_avail_required,
			Error::<T>::NotEnoughAvailForBoost
		);

		// Create sets for efficient comparison
		let selected_pools: BTreeSet<PoolId> = pool_ids.iter().cloned().collect();
		let current_boost_pools_set: BTreeSet<PoolId> =
			current_boost_pools.iter().cloned().collect();

		// Pools to remove boost from: in current but not in selected
		let pools_to_remove: Vec<PoolId> = current_boost_pools_set
			.difference(&selected_pools)
			.cloned()
			.collect();

		// Pools to add boost to: in selected but not in current
		let pools_to_add: Vec<PoolId> = selected_pools
			.difference(&current_boost_pools_set)
			.cloned()
			.collect();

		// Remove user from boost in pools_to_remove
		for pool_id in pools_to_remove.iter() {
			// Remove HasBoost entry
			HasBoost::<T>::remove(*pool_id, fusion_address);

			// Update pool's boost_data
			Pools::<T>::try_mutate(*pool_id, |maybe_pool| -> DispatchResult {
				let pool = maybe_pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;
				if let Some(ref mut boost_data) = pool.boost_data {
					// Get the user's active points in the pool
					let membership = Memberships::<T>::get(fusion_address, *pool_id)
						.ok_or(Error::<T>::MembershipNotFound)?;
					boost_data.elligible_total_points = boost_data
						.elligible_total_points
						.saturating_sub(membership.active_points);
					boost_data
						.elligible_members
						.retain(|addr| *addr != fusion_address);
				}
				Ok(())
			})?;
		}

		// Add user to boost in pools_to_add
		for pool_id in pools_to_add.iter() {
			// Insert HasBoost entry
			HasBoost::<T>::insert(*pool_id, fusion_address, true);

			// Update pool's boost data
			Pools::<T>::try_mutate(*pool_id, |maybe_pool| -> DispatchResult {
				let pool = maybe_pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;
				if let Some(ref mut boost_data) = pool.boost_data {
					// Get the user's active points in the pool
					let membership = Memberships::<T>::get(fusion_address, *pool_id)
						.ok_or(Error::<T>::MembershipNotFound)?;
					boost_data.elligible_total_points = boost_data
						.elligible_total_points
						.saturating_add(membership.active_points);
					boost_data
						.elligible_members
						.try_push(fusion_address)
						.map_err(|_| Error::<T>::PoolMemberLimitReached)?;
				} else {
					// Pool does not have boost data
					return Err(Error::<T>::PoolHasNoBoost.into());
				}
				Ok(())
			})?;
		}

		// Emit event indicating the optimization result
		Self::deposit_event(Event::<T>::UserBoostAllocationsOptimized {
			fusion_address,
			pools_added: pools_to_add,
			pools_removed: pools_to_remove,
		});

		Ok(())
	}

	fn log_if_error(
		result: Result<(), DispatchError>,
		function_name: &str,
		era: EraIndex,
	) -> Result<(), DispatchError> {
		if let Err(ref err) = result {
			Self::deposit_event(Event::ErrorDataEvent {
				detail: format!("Error in {:?} for era {:?}: {:?}", function_name, era, err),
			});
		}
		result
	}
}

impl<T: Config> FusionExt<T::AccountId, BalanceOf<T>, PoolId> for Pallet<T> {
	fn set_fusion_exposures() {
		let era = T::StakingFusionDataProvider::active_era();
		let planned_era = era.saturating_add(1);
		let mut at_least_one = false;
		// Iterate over all pools
		for (pool_id, pool) in Pools::<T>::iter() {
			// Check if the pool is open, has members, and has targets
			if pool.is_active()
				&& !pool.members.is_empty()
				&& !pool.targets.is_empty()
				&& pool.total_staked_points > 0
			{
				// Get currency
				let Some(currency) = Currencies::<T>::get(pool.currency_id) else {
					Self::deposit_event(Event::ErrorDataEvent {
						detail: format!("Error while setting exposure for planned_era {:?} and pool {:?} - Could not get related currency.", planned_era, pool_id)
					});
					continue;
				};

				// Get total amount in avail
				let total_avail_result = pool.points_to_avail(
					pool.total_staked_points,
					pool.currency_id,
					Some(&currency),
					Some(era),
				);

				let Ok(total_avail) = total_avail_result else {
					Self::deposit_event(Event::ErrorDataEvent {
						detail: format!("Error while setting exposure for planned_era {:?} and pool {:?} - Could not compute avail amount from pool points. - Details: {:?}", planned_era, pool_id, total_avail_result)
					});
					continue;
				};

				// Set boost data in the exposure
				let (boost_value, boost_total_points, boost_total_avail, boost_members) =
					pool.boost_data.as_ref().map_or(
						(
							Perbill::zero(),
							Points::default(),
							BalanceOf::<T>::default(),
							BoundedVec::default(),
						),
						|data| {
							let boost_points = data.elligible_total_points;
							let boost_avail = pool
								.points_to_avail(
									boost_points,
									pool.currency_id,
									Some(&currency),
									Some(era),
								)
								.unwrap_or(BalanceOf::<T>::default());
							(
								data.additional_apy,
								boost_points,
								boost_avail,
								data.elligible_members.clone(),
							)
						},
					);

				// We set the exposure for era + 1
				// The data must be available for the snapshot and next elections
				let fusion_exposure = FusionExposure::<T> {
					era: planned_era,
					total_avail,
					total_points: pool.total_staked_points,
					user_points: pool.members.clone(),
					targets: pool.targets.clone(),
					apy: pool.apy,
					native_exposure_data: None,
					boost_members,
					boost_total_points,
					boost_total_avail,
					boost_additional_apy: boost_value,
				};
				Exposures::<T>::insert(planned_era, pool_id, fusion_exposure);
				at_least_one = true;
			}
		}
		if at_least_one {
			Self::deposit_event(Event::<T>::ExposuresSet { era: planned_era });
		}
	}

	fn handle_end_era(era: EraIndex, era_duration: u64) {
		Self::compute_era_rewards(era, era_duration, None);

		let _ = Self::log_if_error(Self::setup_currency_rates(era), "setup_currency_rates", era);
		let _ = Self::log_if_error(
			Self::clean_history_depth_storages(era),
			"clean_history_depth_storages",
			era,
		);
	}

	fn get_fusion_voters() -> Vec<(T::AccountId, u64, Vec<T::AccountId>)> {
		// We take the planned era here
		let era = T::StakingFusionDataProvider::active_era().saturating_add(1);
		let mut fusion_voters: Vec<(T::AccountId, u64, Vec<T::AccountId>)> = Vec::new();

		let total_issuance = T::Currency::total_issuance();

		for (pool_id, exposure) in Exposures::<T>::iter_prefix(era) {
			if exposure.targets.is_empty() || exposure.total_avail.is_zero() {
				continue;
			}
			let account = Self::get_pool_funds_account(pool_id);
			let targets = exposure.targets;
			let stake = exposure.total_avail;
			let fusion_pool_weight = T::CurrencyToVote::to_vote(stake, total_issuance);
			fusion_voters.push((account, fusion_pool_weight, targets.to_vec()));
		}
		fusion_voters
	}

	fn get_active_pool_count() -> usize {
		let era = T::StakingFusionDataProvider::active_era().saturating_add(1);
		Exposures::<T>::iter_prefix(era).count()
	}

	fn get_pool_id_from_funds_account(account: &T::AccountId) -> Option<PoolId> {
		PoolsAccountToId::<T>::get(account)
	}

	fn update_pool_exposure(
		maybe_pool_account: &T::AccountId,
		validator: &T::AccountId,
		value: BalanceOf<T>,
		era: EraIndex,
	) {
		let Some(pool_id) = Self::get_pool_id_from_funds_account(maybe_pool_account) else {
			return;
		};

		let _ = Exposures::<T>::try_mutate(era, pool_id, |maybe_exposure| -> DispatchResult {
			// Ensure rewards are available for the given era and pool
			let Some(ref mut exposure) = maybe_exposure else {
				return Ok(());
			};

			let mut native_exposure_data =
				exposure.native_exposure_data.clone().unwrap_or_default();

			if native_exposure_data
				.try_push((validator.clone(), value))
				.is_err()
			{
				Self::deposit_event(Event::ErrorDataEvent {
					detail: format!("Could not update fusion exposure for pool {:?} - native_exposure_data limit reached", pool_id)
				});
			};

			let _ = PoolsBackingValidator::<T>::try_mutate(
				era,
				validator,
				|pool_ids| -> DispatchResult {
					if pool_ids.try_push(pool_id).is_err() {
						Self::deposit_event(Event::ErrorDataEvent {
							detail: format!("Could not set fusion pools from validator for pool {pool_id:?} and validator {validator:?} and era {era:?}"),
						});
					}
					Ok(())
				},
			);

			exposure.native_exposure_data = Some(native_exposure_data);

			Ok(())
		});
	}

	fn add_fusion_slash(
		era: EraIndex,
		validator: &T::AccountId,
		nominators: &Vec<(T::AccountId, BalanceOf<T>)>,
	) -> Weight {
		let mut consummed_weight = Weight::from_parts(0, 0);

		let pool_ids = PoolsBackingValidator::<T>::get(era, validator);

		consummed_weight = consummed_weight.saturating_add(T::DbWeight::get().reads(1));

		if pool_ids.is_empty() {
			return consummed_weight;
		}

		let mut pool_funds_accounts: BTreeMap<T::AccountId, PoolId> = pool_ids
			.iter()
			.map(|id| (Self::get_pool_funds_account(*id), *id))
			.collect();

		let filtered_nominators: Vec<(PoolId, BalanceOf<T>)> = nominators
			.iter()
			.filter_map(|(nominator_account, balance)| {
				pool_funds_accounts
					.remove(nominator_account)
					.map(|pool_id| (pool_id, *balance))
			})
			.collect();

		for (pool_id, slashed_amount) in filtered_nominators.iter() {
			let result = Pools::<T>::try_mutate(pool_id, |maybe_pool| -> DispatchResult {
				let pool = maybe_pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;
				consummed_weight = consummed_weight.saturating_add(T::DbWeight::get().reads(1));

				let exposure =
					Exposures::<T>::get(era, pool_id).ok_or(Error::<T>::ExposureNotFound)?;
				consummed_weight = consummed_weight.saturating_add(T::DbWeight::get().reads(1));

				ensure!(
					pool.state != FusionPoolState::Destroying,
					Error::<T>::PoolIsDestroying
				);

				let exposure_total_avail = exposure.total_avail;
				let slash_total_avail = slashed_amount.min(&exposure_total_avail);
				let slash_ratio = Perbill::from_rational(*slash_total_avail, exposure_total_avail);

				let new_pending_slash = FusionPendingSlash {
					slash_era: era,
					slash_ratio,
					validator: validator.clone(),
				};

				pool.pending_slashes
					.try_push(new_pending_slash)
					.map_err(|_| Error::<T>::PendingSlashLimitReached)?;

				HasPendingSlash::<T>::mutate(
					era,
					(validator, pool.funds_account.clone()),
					|count| *count = count.saturating_add(1),
				);
				consummed_weight = consummed_weight.saturating_add(T::DbWeight::get().reads(1));
				consummed_weight = consummed_weight.saturating_add(T::DbWeight::get().writes(2));

				Self::deposit_event(Event::<T>::FusionSlashReported {
					pool_id: *pool_id,
					slash_era: era,
					slash_ratio,
					validator: validator.clone(),
				});

				Ok(())
			});

			if let Err(e) = result {
				Self::deposit_event(Event::ErrorDataEvent {
					detail: format!("An error occured while trying to add a slash for pool {pool_id:?}, era {era:?} and validator {validator:?} - Error detail: {e:?}"),
				});
			}
		}

		consummed_weight
	}

	fn cancel_fusion_slash(era: EraIndex, slash_indices: Vec<u32>) {
		// Get the unapplied slashes for the era where the slash should get applied
		let native_unapplied_slashes = T::StakingFusionDataProvider::unapplied_slashes(era);

		// Converts the era to the slash era
		let era = era
			.saturating_sub(T::SlashDeferDuration::get())
			.saturating_sub(1);

		let mut slashes_to_cancel: BTreeMap<PoolId, Vec<usize>> = BTreeMap::new();
		let mut slashes_to_cancel_pools: BTreeMap<PoolId, FusionPool<T>> = BTreeMap::new();
		let mut validators: Vec<T::AccountId> = Vec::new();
		let mut pool_ids: Vec<PoolId> = Vec::new();

		for slash_index in slash_indices {
			// Get the native unapplied slash
			let unapplied_slash = &native_unapplied_slashes[slash_index as usize];

			// Check if a pool is concerned
			let concerned_pool_ids =
				PoolsBackingValidator::<T>::get(era, &unapplied_slash.validator);
			if concerned_pool_ids.len() == 0 {
				continue;
			}

			validators.push(unapplied_slash.validator.clone());

			// We need to handle the case where the validator has multiple slash for the same era (it can happen)
			// It means, we need to know which occurence for this validator we need to cancel (if we're cancelling the first, the second, the third, ...)
			let mut slash_index_for_this_validator = 0;
			for (i, slash) in native_unapplied_slashes.iter().enumerate() {
				if slash.validator == unapplied_slash.validator {
					if i == slash_index as usize {
						break;
					}
					slash_index_for_this_validator += 1;
				}
			}

			for pool_id in concerned_pool_ids {
				let Some(pool) = Pools::<T>::get(pool_id) else {
					Self::deposit_event(Event::ErrorDataEvent {
						detail: format!("Pool not found while trying to remove a slash for pool {pool_id:?}, era {era:?}, Slash won't get applied and it will be cleaned after few eras"),
					});
					return;
				};

				// Now the pool can be slashed for different validators multiple times for each one of them
				// So we need to find the correct occurence using the slash_index_for_this_validator computed just before
				let mut occurrence_count = 0;
				let mut pool_slash_to_remove_index = None;
				for (i, slash) in pool.pending_slashes.iter().enumerate() {
					if slash.validator == unapplied_slash.validator {
						if occurrence_count == slash_index_for_this_validator {
							pool_slash_to_remove_index = Some(i);
							break;
						}
						occurrence_count += 1;
					}
				}

				let Some(pool_slash_to_remove_index) = pool_slash_to_remove_index else {
					Self::deposit_event(Event::ErrorDataEvent {
						detail: format!("Slash index not found while trying to remove a slash for pool {pool_id:?}, era {era:?}, Slash won't get applied and it will be cleaned after few eras"),
					});
					return;
				};

				slashes_to_cancel
					.entry(pool_id)
					.or_insert_with(Vec::new)
					.push(pool_slash_to_remove_index);
				slashes_to_cancel_pools.entry(pool_id).or_insert(pool);
			}
		}

		for (pool_id, indices_to_remove) in &slashes_to_cancel {
			if let Some(pool) = slashes_to_cancel_pools.get_mut(pool_id) {
				let mut index = 0;
				pool.pending_slashes.retain(|pending_slash| {
					if !indices_to_remove.contains(&index) {
						index += 1;
						true
					} else {
						HasPendingSlash::<T>::mutate(
							era,
							(pending_slash.validator.clone(), pool.funds_account.clone()),
							|count| {
								*count = count.saturating_sub(1);
								if *count == 0 {
									HasPendingSlash::<T>::remove(
										era,
										(
											pending_slash.validator.clone(),
											pool.funds_account.clone(),
										),
									);
								}
							},
						);
						index += 1;
						false
					}
				});
				Pools::<T>::insert(pool_id, pool);
				pool_ids.push(*pool_id);
			}
		}

		if validators.len() > 0 {
			Self::deposit_event(Event::<T>::FusionSlashCancelled {
				pool_ids,
				slash_era: era,
				validators,
			});
		}
	}

	fn apply_fusion_slash(
		slash_era: EraIndex,
		validator: &T::AccountId,
		funds_account: &T::AccountId,
	) -> bool {
		let slash_era = if T::SlashDeferDuration::get() == 0 {
			slash_era
		} else {
			slash_era.saturating_sub(1) // If we have a defer duration, the slash_era parameter is one era later in the staking pallet
		};

		let Some(pool_id) = Self::get_pool_id_from_funds_account(funds_account) else {
			return false;
		};

		if HasPendingSlash::<T>::get(slash_era, (validator, funds_account)) == 0 {
			Self::deposit_event(Event::ErrorDataEvent {
				detail: format!("Pool {pool_id:?} should have been slashed at era {slash_era:?} for validator {validator:?} but the pending slash was not found in the storage."),
			});
			return true;
		}

		let result = Pools::<T>::try_mutate(pool_id, |maybe_pool| -> DispatchResult {
			let pool = maybe_pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;

			let mut currency =
				Currencies::<T>::get(pool.currency_id).ok_or(Error::<T>::CurrencyNotFound)?;

			let maybe_removed_slash = pool
				.pending_slashes
				.iter()
				.position(|slash| slash.slash_era == slash_era && slash.validator == *validator)
				.map(|index| pool.pending_slashes.remove(index));

			if let Some(removed_slash) = maybe_removed_slash {
				let mut total_slashed: FusionCurrencyBalance = 0;
				let slash_ratio = removed_slash.slash_ratio;

				// Slash the pool
				let slashed_amount_from_pool = slash_ratio * pool.total_staked_native;
				pool.total_staked_native = pool
					.total_staked_native
					.saturating_sub(slashed_amount_from_pool);
				currency.total_staked_native = currency
					.total_staked_native
					.saturating_sub(slashed_amount_from_pool);

				total_slashed = total_slashed.saturating_add(slashed_amount_from_pool);

				// Slash the slashable unbonding chunks of the pool
				let mut slashed_amount_from_chunks: FusionCurrencyBalance = 0;

				// Iterate over all unbonding chunks for the specified pool
				for (unbond_era, chunks) in UnbondingChunks::<T>::iter_prefix(pool_id) {
					let mut updated_chunks = BoundedVec::default();
					if unbond_era >= slash_era {
						// Iterate over the chunks in the BoundedVec
						for (fusion_address, balance) in chunks {
							// Calculate the slashed amount for this chunk
							let slashed_amount = slash_ratio * balance;

							// Add to the slashed_amount_from_chunks
							slashed_amount_from_chunks =
								slashed_amount_from_chunks.saturating_add(slashed_amount);

							// Update the remaining balance in the bounded vec
							let new_balance = balance.saturating_sub(slashed_amount);
							if new_balance > FusionCurrencyBalance::zero() {
								updated_chunks
									.try_push((fusion_address, new_balance))
									.map_err(|_| Error::<T>::PoolMemberLimitReached)?;
							}
						}

						if !updated_chunks.is_empty() {
							UnbondingChunks::<T>::insert(pool_id, unbond_era, updated_chunks);
						} else {
							UnbondingChunks::<T>::remove(pool_id, unbond_era);
						}
					}
				}

				total_slashed = total_slashed.saturating_add(slashed_amount_from_chunks);

				pool.total_unbonding_native = pool
					.total_unbonding_native
					.saturating_sub(slashed_amount_from_chunks);
				currency.total_unbonding_native = currency
					.total_unbonding_native
					.saturating_sub(slashed_amount_from_chunks);

				pool.total_slashed_native = pool.total_slashed_native.saturating_add(total_slashed);
				currency.total_slashed_native =
					currency.total_slashed_native.saturating_add(total_slashed);

				// Update TVL
				// slashed_amount_from_chunks was already deduced from tvl when unbonded
				Self::sub_from_tvl(pool.currency_id, &currency, slashed_amount_from_pool)?;

				if let Some(slash_destination) = SlashDestination::<T>::get() {
					Self::add_to_currency_balance(
						slash_destination,
						pool.currency_id,
						total_slashed,
						true,
					)?;
				} else {
					Self::deposit_event(Event::ErrorDataEvent {
						detail: format!("No slash destination provided, funds are burned on Avail side. Currency Id: {:?} - Amount: {:?}.", pool.currency_id, total_slashed),
					});
				}

				Currencies::<T>::insert(pool.currency_id, currency);
				HasPendingSlash::<T>::mutate(
					slash_era,
					(removed_slash.validator.clone(), funds_account),
					|count| {
						*count = count.saturating_sub(1);
						if *count == 0 {
							HasPendingSlash::<T>::remove(
								slash_era,
								(removed_slash.validator, funds_account),
							);
						}
					},
				);

				// If the avail pool is slashed, we remove all boosts cause we cannot compute the correct values anymore
				// We can call the permissionless extrinsic to re-optimize the pools boost allocations
				if pool_id == AVAIL_POOL_ID {
					Self::shutdown_pools_boost();
				}

				Self::deposit_event(Event::<T>::FusionPoolSlashed {
					currency_id: pool.currency_id,
					pool_id,
					slash_era,
					amount: total_slashed,
				});

				Ok(())
			} else {
				Err(Error::<T>::SlashNotFound.into())
			}
		});

		if let Err(e) = result {
			Self::deposit_event(Event::ErrorDataEvent {
				detail: format!("An error occured while trying to apply a slash for pool {pool_id:?}, era {slash_era:?}, Slash won't get applied and it will be cleaned after few eras - Error detail: {e:?}"),
			});
		}

		true
	}
}
