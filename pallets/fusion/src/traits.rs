use crate::*;
use pallet_staking::UnappliedSlash;
use sp_staking::EraIndex;

// A trait that provides data from the staking pallet.
pub trait StakingFusionDataProvider<T: Config> {
	/// Returns the active era.
	fn active_era() -> EraIndex;
	/// Returns the currently planned era.
	fn current_era() -> EraIndex;
	/// Checks if an account is a validator.
	fn is_valid_validator(account: &T::AccountId) -> bool;
	/// Checks if a validator has earned era points for an era (meaning he'll get rewards).
	fn has_earned_era_points(era: EraIndex, accounts: &Vec<T::AccountId>) -> bool;
	/// Gets the unapplied slashes for an era
	fn unapplied_slashes(era: EraIndex) -> Vec<UnappliedSlash<T::AccountId, BalanceOf<T>>>;
	/// Adds a dummy validator to both the Session and Staking pallets.
	#[cfg(feature = "runtime-benchmarks")]
	fn add_dummy_validator(account: T::AccountId);
	/// Adds dummy era point for a validator and an era
	#[cfg(feature = "runtime-benchmarks")]
	fn add_dummy_era_points(validator: T::AccountId, era: EraIndex);
	/// Set the active era
	#[cfg(feature = "runtime-benchmarks")]
	fn set_dummy_active_era(era: EraIndex);
}
impl<T: Config> StakingFusionDataProvider<T> for () {
	fn active_era() -> EraIndex {
		0
	}
	fn current_era() -> EraIndex {
		0
	}
	fn is_valid_validator(_account: &T::AccountId) -> bool {
		false
	}
	fn has_earned_era_points(_era: EraIndex, _accounts: &Vec<T::AccountId>) -> bool {
		false
	}
	fn unapplied_slashes(_era: EraIndex) -> Vec<UnappliedSlash<T::AccountId, BalanceOf<T>>> {
		Vec::new()
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add_dummy_validator(_account: T::AccountId) {
		()
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add_dummy_era_points(_validator: T::AccountId, _era: EraIndex) {
		()
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn set_dummy_active_era(_era: EraIndex) {
		()
	}
}

pub trait PoolAccountProvider<T: Config> {
	/// Return the pool funds account
	fn get_pool_funds_account(id: PoolId) -> T::AccountId;
	/// Return the pool claimable account
	fn get_pool_claimable_account(id: PoolId) -> T::AccountId;
}
impl<T: Config> PoolAccountProvider<T> for () {
	fn get_pool_funds_account(id: PoolId) -> T::AccountId {
		T::PalletId::get().into_sub_account_truncating((FusionAccountType::PoolFundsAccount, id))
	}
	fn get_pool_claimable_account(id: PoolId) -> T::AccountId {
		T::PalletId::get()
			.into_sub_account_truncating((FusionAccountType::PoolClaimableAccount, id))
	}
}
