use codec::Codec;
use frame_support::pallet_prelude::*;
use sp_core::{H160, H256, H512};
use sp_runtime::Perbill;
use sp_staking::EraIndex;

use crate::*;

/// Type representing a fusion address (for now we use H160 as it's EVM compatible)
#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum FusionAddress {
	EvmAddress(H160),    // 20-byte Ethereum-like address
	SolanaAddress(H256), // 32-byte Solana address
	Other(H512),         // 64-byte general-purpose identifier
}
impl FusionAddress {
	/// Create a new Evm Address
	pub fn new_evm(address: H160) -> Self {
		FusionAddress::EvmAddress(address)
	}
	/// Create a new Solana Address
	pub fn new_solana(address: H256) -> Self {
		FusionAddress::SolanaAddress(address)
	}
	/// Create a new Other address
	pub fn new_other(address: H512) -> Self {
		FusionAddress::Other(address)
	}
}

/// Type representing a balance for external currency
pub type FusionCurrencyBalance = u128;

/// Type to represent points
pub type Points = u128;

/// Type of the currency id
pub type CurrencyId = u32;

/// Type of the pool id
pub type PoolId = u32;

/// Possible operations on the configuration values of this pallet.
#[derive(TypeInfo, Debug, Clone, Encode, Decode, PartialEq)]
pub enum ConfigOp<T: Codec> {
	/// Don't change.
	Noop,
	/// Set the given value.
	Set(T),
	/// Remove the value.
	Remove,
}

/// The type of account being created.
#[derive(Encode, Decode)]
pub enum FusionAccountType {
	AvailCurrency,
	PoolFundsAccount,
	PoolClaimableAccount,
}

/// State of the pool
#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum FusionPoolState {
	/// Anyone can join, the pool is earning rewards
	Open,
	/// Nobody can join, the pool is earning rewards
	Blocked,
	/// Nobody can join, the pool is not earning rewards
	Paused,
	/// Pool is getting deleted, nobody can join, the pool is not earning rewards
	Destroying,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct FusionCurrency<T: Config> {
	/// Name of the currency (e.g., "AVAIL", "ETH", "wBTC")
	pub name: BoundedVec<u8, T::MaxCurrencyNameLength>,
	/// Number of decimals to represent 1 unit of the currency (e.g., 8 for wBTC, 18 for ETH)
	pub nb_decimals: u8,
	/// The amount staked in native form
	pub total_staked_native: FusionCurrencyBalance,
	/// The amount slashed in native form
	pub total_slashed_native: FusionCurrencyBalance,
	/// The amount unbonding in native form
	pub total_unbonding_native: FusionCurrencyBalance,
	/// Maximum allowable stake for this currency (overall)
	pub max_amount: FusionCurrencyBalance,
	/// Minimum amount to join a pool of this currency
	pub min_amount: FusionCurrencyBalance,
	/// State of the currency
	pub is_destroyed: bool,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct FusionPool<T: Config> {
	/// Id of the currency this pool uses
	pub currency_id: CurrencyId,
	/// Percentage representing annual yield for this pool
	pub apy: Perbill,
	/// The account used during snapshot and for Phragmen, this account will receive rewards, this account can be topped up
	pub funds_account: T::AccountId,
	/// The account used to store claimable avail
	pub claimable_account: T::AccountId,
	/// Optional nominator of the pool, mandate can always manage
	pub nominator: Option<T::AccountId>,
	/// The Fusion addresses of members of the pool
	pub members: BoundedVec<(FusionAddress, Points), T::MaxMembersPerPool>,
	/// The target validators to be nominated by this pool
	pub targets: BoundedVec<T::AccountId, T::MaxTargets>,
	/// The amount staked in native form
	pub total_staked_native: FusionCurrencyBalance,
	/// The amount staked in points
	pub total_staked_points: Points,
	/// The amount slashed in this pool
	pub total_slashed_native: FusionCurrencyBalance,
	/// The total amount unbonding in this pool, conversion happens at unbonding
	pub total_unbonding_native: FusionCurrencyBalance,
	/// State of the pool
	pub state: FusionPoolState,
	/// Vector of pending slashes
	pub pending_slashes: BoundedVec<FusionPendingSlash<T>, T::MaxSlashesPerPool>,
	/// Data about boost
	pub boost_data: Option<BoostData<T>>,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct BoostData<T: Config> {
	/// The additional apy on the pool
	pub additional_apy: Perbill,
	/// The minimum avail that needs to be allocated to this pool to earn boost
	pub min_avail_to_earn: FusionCurrencyBalance,
	/// The amount of points in the pool getting boost
	pub elligible_total_points: Points,
	/// Vector with elligible members
	pub elligible_members: BoundedVec<FusionAddress, T::MaxMembersPerPool>,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct FusionMembership<T: Config> {
	/// Fusion address of the user
	pub fusion_address: FusionAddress,
	/// Era where the membership was created
	pub joined_era: EraIndex,
	/// The stake of the user represented by points
	pub active_points: Points,
	/// Unbonding eras of the user
	pub unbonding_eras: BoundedVec<EraIndex, T::MaxUnbonding>,
	/// If true, rewards will go to the AVAIL pool
	pub is_compounding: bool,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct FusionUserCurrencyBalance {
	/// Fusion address of the user
	pub fusion_address: FusionAddress,
	/// Id of the idle currency this pool uses
	pub currency_id: CurrencyId,
	/// Amount of currency available, for AVAIL, it's the amount you can compound
	pub amount: FusionCurrencyBalance,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct FusionExposure<T: Config> {
	/// Era of the exposure to compute rewards
	pub era: EraIndex,
	/// The APY when the exposure was taken
	pub apy: Perbill,
	/// The total in avail
	pub total_avail: BalanceOf<T>,
	/// The total points in the pool
	pub total_points: Points,
	/// The users points in the pool
	pub user_points: BoundedVec<(FusionAddress, Points), T::MaxMembersPerPool>,
	/// The nominations of the pool at the time of setting the exposure
	pub targets: BoundedVec<T::AccountId, T::MaxTargets>,
	/// Used to store the validator(s) actually backed alongside the amount
	/// This is populated when exposure are collected
	pub native_exposure_data: Option<BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxTargets>>,
	/// Boost additional APY
	pub boost_additional_apy: Perbill,
	/// The members having boost
	pub boost_members: BoundedVec<FusionAddress, T::MaxMembersPerPool>,
	/// The total points elligible to boost
	pub boost_total_points: Points,
	/// The avail equivalent of boost_total_points
	pub boost_total_avail: BalanceOf<T>,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct EraReward<T: Config> {
	/// The total rewards
	pub rewards: BalanceOf<T>,
	/// The actual amount of reward claimed
	pub claimed_rewards: BalanceOf<T>,
	/// The total rewards from boost
	pub additional_rewards: BalanceOf<T>,
	/// The actual amount of reward claimed from boost
	pub additional_claimed_rewards: BalanceOf<T>,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct FusionPendingSlash<T: Config> {
	/// Era when the slash happened
	pub slash_era: EraIndex,
	/// Percentage of the pool funds that got slashed
	pub slash_ratio: Perbill,
	/// The validator that got slashed
	pub validator: T::AccountId,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct TVLData<T: Config> {
	/// The total value locked in Fusion from users (in avail)
	pub total_value_locked: BalanceOf<T>,
	/// The max total allowed values locked in Fusion (when changing conversion rates or staking new currency, this will be checked)
	pub max_total_value_locked: BalanceOf<T>,
}

impl<T: Config> FusionCurrency<T> {
	/// Converts a given amount of this external currency to its equivalent in AVAIL.
	pub fn currency_to_avail(
		&self,
		currency_id: CurrencyId,
		amount: FusionCurrencyBalance,
		era: Option<EraIndex>,
	) -> Result<BalanceOf<T>, Error<T>> {
		let rate = CurrencyRates::<T>::get(
			era.unwrap_or_else(T::StakingFusionDataProvider::active_era),
			currency_id,
		)
		.ok_or(Error::<T>::CurrencyRateNotFound)?;

		let rate = Pallet::<T>::u256(rate.try_into().map_err(|_| Error::<T>::ArithmeticError)?);
		let amount = Pallet::<T>::u256(amount);
		let divisor = Pallet::<T>::u256(10u128.pow(self.nb_decimals as u32));

		let avail_value = rate
			.saturating_mul(amount)
			.checked_div(divisor)
			.ok_or(Error::<T>::ArithmeticError)?;

		Ok(Pallet::<T>::balance(avail_value))
	}

	/// Converts a given amount of AVAIL to its equivalent in this external currency.
	pub fn avail_to_currency(
		&self,
		currency_id: CurrencyId,
		avail_amount: BalanceOf<T>,
		era: Option<EraIndex>,
	) -> Result<FusionCurrencyBalance, Error<T>> {
		let era = era.unwrap_or_else(T::StakingFusionDataProvider::active_era);

		let rate =
			CurrencyRates::<T>::get(era, currency_id).ok_or(Error::<T>::CurrencyRateNotFound)?;

		let rate = Pallet::<T>::u256(rate.try_into().map_err(|_| Error::<T>::ArithmeticError)?);
		let avail_amount = Pallet::<T>::u256(
			avail_amount
				.try_into()
				.map_err(|_| Error::<T>::ArithmeticError)?,
		);
		let multiplier = Pallet::<T>::u256(10u128.pow(self.nb_decimals as u32));

		let currency_value = avail_amount
			.saturating_mul(multiplier)
			.checked_div(rate)
			.ok_or(Error::<T>::ArithmeticError)?;

		Ok(Pallet::<T>::fusion_currency(currency_value))
	}
}

impl<T: Config> FusionPool<T> {
	/// Helper to check if the pool is in Open state
	pub fn is_active(&self) -> bool {
		self.state == FusionPoolState::Open || self.state == FusionPoolState::Blocked
	}
	/// Converts a given amount of points to its equivalent in external currency.
	pub fn points_to_currency(
		&self,
		points: Points,
		currency: Option<&FusionCurrency<T>>,
	) -> Result<FusionCurrencyBalance, Error<T>> {
		if self.total_staked_native == 0 && self.total_staked_points == 0 {
			let currency_decimals = if let Some(c) = currency {
				c.nb_decimals
			} else {
				let stored_currency =
					Currencies::<T>::get(self.currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
				stored_currency.nb_decimals
			};

			let divisor = Pallet::<T>::u256(10u128.pow(18 - currency_decimals as u32));
			let points = Pallet::<T>::u256(points);

			let currency_value = points
				.checked_div(divisor)
				.ok_or(Error::<T>::ArithmeticPointsError)?;

			Ok(Pallet::<T>::fusion_currency(currency_value))
		} else {
			ensure!(
				self.total_staked_points > 0,
				Error::<T>::ArithmeticPointsError
			);

			let points = Pallet::<T>::u256(points);
			let total_staked_native = Pallet::<T>::u256(self.total_staked_native);
			let total_staked_points = Pallet::<T>::u256(self.total_staked_points);

			let currency_value = points
				.saturating_mul(total_staked_native)
				.checked_div(total_staked_points)
				.ok_or(Error::<T>::ArithmeticPointsError)?;

			Ok(Pallet::<T>::fusion_currency(currency_value))
		}
	}
	/// Converts a given amount of external currency to its equivalent in points.
	pub fn currency_to_points(
		&self,
		currency_amount: FusionCurrencyBalance,
		currency: Option<&FusionCurrency<T>>,
	) -> Result<Points, Error<T>> {
		if self.total_staked_native == 0 && self.total_staked_points == 0 {
			let currency_decimals = if let Some(c) = currency {
				c.nb_decimals
			} else {
				let currency =
					Currencies::<T>::get(self.currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
				currency.nb_decimals
			};
			let multiplier = Pallet::<T>::u256(10u128.pow(18 - currency_decimals as u32));
			let currency_amount = Pallet::<T>::u256(currency_amount);

			let points = currency_amount.saturating_mul(multiplier);

			Ok(Pallet::<T>::points(points))
		} else {
			ensure!(
				self.total_staked_native > 0,
				Error::<T>::ArithmeticPointsError
			);

			let currency_amount = Pallet::<T>::u256(currency_amount);
			let total_staked_native = Pallet::<T>::u256(self.total_staked_native);
			let total_staked_points = Pallet::<T>::u256(self.total_staked_points);

			let points = currency_amount
				.saturating_mul(total_staked_points)
				.checked_div(total_staked_native)
				.ok_or(Error::<T>::ArithmeticPointsError)?;

			Ok(Pallet::<T>::points(points))
		}
	}
	/// Converts a given amount of points to its equivalent in AVAIL.
	pub fn points_to_avail(
		&self,
		points: Points,
		currency_id: CurrencyId,
		currency: Option<&FusionCurrency<T>>,
		era: Option<EraIndex>,
	) -> Result<BalanceOf<T>, Error<T>> {
		let currency_value = self.points_to_currency(points, currency)?;

		let avail_value = if let Some(currency) = currency {
			currency.currency_to_avail(currency_id, currency_value, era)?
		} else {
			let currency =
				Currencies::<T>::get(self.currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
			currency.currency_to_avail(currency_id, currency_value, era)?
		};

		Ok(avail_value)
	}
	/// Converts a given amount of AVAIL to its equivalent in points.
	pub fn avail_to_points(
		&self,
		avail_amount: BalanceOf<T>,
		currency_id: CurrencyId,
		currency: Option<&FusionCurrency<T>>,
		era: Option<EraIndex>,
	) -> Result<Points, Error<T>> {
		let currency_value = if let Some(currency) = currency {
			currency.avail_to_currency(currency_id, avail_amount, era)?
		} else {
			let currency = Currencies::<T>::get(currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
			currency.avail_to_currency(currency_id, avail_amount, era)?
		};

		let points = self.currency_to_points(currency_value, currency)?;
		Ok(points)
	}

	pub fn set_boost(
		&mut self,
		pool_id: PoolId,
		boost_data: Option<(Perbill, FusionCurrencyBalance)>,
	) -> DispatchResult {
		match (&self.boost_data, boost_data) {
			(None, None) => {
				// There is no current boost, nothing to do
			},
			(Some(_old_boost), None) => {
				// There is some boost, we remove it
				// We remove the pool id from the storage of pools with boost
				PoolsWithBoost::<T>::remove(pool_id);

				// We remove all the users for this pool in HasBoost
				let _ = HasBoost::<T>::clear_prefix(pool_id, u32::MAX, None);

				// We update the pool
				self.boost_data = None
			},
			(None, Some((apy, min_to_earn))) => {
				// There is no current boost, we add it
				// We add the pool the to vec of pools having boost
				PoolsWithBoost::<T>::insert(pool_id, min_to_earn);

				// We update the pool
				self.boost_data = Some(BoostData {
					additional_apy: apy,
					min_avail_to_earn: min_to_earn,
					elligible_total_points: 0,
					elligible_members: BoundedVec::default(),
				});
			},
			(Some(old_boost), Some((apy, min_to_earn))) => {
				// There is already a boost, we update it
				PoolsWithBoost::<T>::insert(pool_id, min_to_earn);

				// For each users having boost,
				// we need to check if they still belong.
				// But it's too expensive to do onchain so we leave them in.
				// If we need to clean it, we can remove the boost
				// and call the permissionless extrinsic to optimize
				self.boost_data = Some(BoostData {
					additional_apy: apy,
					min_avail_to_earn: min_to_earn,
					elligible_total_points: old_boost.elligible_total_points,
					elligible_members: old_boost.elligible_members.clone(),
				});
			},
		};
		Ok(())
	}
}

impl<T: Config> Default for TVLData<T> {
	fn default() -> Self {
		Self {
			total_value_locked: BalanceOf::<T>::default(),
			max_total_value_locked: BalanceOf::<T>::default(),
		}
	}
}

impl<T: Config> TVLData<T> {
	/// Checks if adding `amount` to `total_value_locked` is within `max_total_value_locked`.
	pub fn can_add(&self, amount: BalanceOf<T>) -> bool {
		self.total_value_locked.saturating_add(amount) <= self.max_total_value_locked
	}

	/// Adds `amount` to `total_value_locked` if it doesn't exceed `max_total_value_locked`.
	/// Returns `Ok(())` if successful, or an error if the addition exceeds the max value.
	pub fn add(&mut self, amount: BalanceOf<T>) -> Result<(), Error<T>> {
		ensure!(self.can_add(amount), Error::<T>::MaxTVLReached);
		self.total_value_locked = self.total_value_locked.saturating_add(amount);
		Ok(())
	}

	/// Substract `amount` to `total_value_locked`.
	pub fn sub(&mut self, amount: BalanceOf<T>) {
		self.total_value_locked = self.total_value_locked.saturating_sub(amount);
	}
}
