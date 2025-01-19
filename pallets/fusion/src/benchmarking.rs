#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Fusion;
use frame_benchmarking::{
	account as benchmark_account, impl_benchmark_test_suite, v1::BenchmarkError, v2::*,
};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_core::H160;
use sp_runtime::SaturatedConversion;

fn get_account<T: Config>(name: &'static str) -> T::AccountId {
	let account: T::AccountId = benchmark_account(name, 0, 0);
	account
}

fn init_benchmarks<T: Config>() {
	let alice: T::AccountId = get_account::<T>("ALICE");
	let bob: T::AccountId = get_account::<T>("BOB");
	let charlie: T::AccountId = get_account::<T>("CHARLIE");
	let dave: T::AccountId = get_account::<T>("DAVE");
	let eve: T::AccountId = get_account::<T>("EVE");
	let ferdie: T::AccountId = get_account::<T>("FERDIE");

	let balance: BalanceOf<T> = 10_000_000_000_000_000_000_000u128.saturated_into();

	T::Currency::make_free_balance_be(
		&Fusion::<T>::avail_account(),
		100_000_000_000_000_000_000_000u128.saturated_into(),
	);
	T::Currency::make_free_balance_be(&alice, balance);
	T::Currency::make_free_balance_be(&bob, balance);
	T::Currency::make_free_balance_be(&charlie, balance);
	T::Currency::make_free_balance_be(&dave, balance);
	T::Currency::make_free_balance_be(&eve, balance);
	T::Currency::make_free_balance_be(&ferdie, balance);

	let origin: T::RuntimeOrigin = RawOrigin::Root.into();
	assert_ok!(Fusion::<T>::update_max_tvl(
		origin.clone(),
		10_000_000_000_000_000_000_000_000u128.saturated_into()
	));
	let slash_destination = FusionAddress::EvmAddress(H160::repeat_byte(0x09));
	assert_ok!(Fusion::<T>::set_slash_destination(
		origin.clone(),
		Some(slash_destination),
		Some(ferdie)
	));

	T::StakingFusionDataProvider::add_dummy_validator(get_account::<T>("BOB"));
}

fn create_avail_currency<T: Config>() {
	let origin = RawOrigin::Root;
	let currency_id = 0;
	let nb_decimals = 18;
	let max_amount = 1_000_000_000_000_000_000_000_000;
	let min_amount = 0;
	let initial_conversion_rate: BalanceOf<T> = 1_000_000_000_000_000_000u128.saturated_into();
	let name: BoundedVec<u8, T::MaxCurrencyNameLength> = b"Avail".to_vec().try_into().unwrap();

	assert_ok!(Fusion::<T>::create_currency(
		origin.into(),
		currency_id,
		name,
		nb_decimals,
		max_amount,
		min_amount,
		initial_conversion_rate,
	));
}

fn create_avail_pool<T: Config>() {
	let origin = RawOrigin::Root;
	let pool_id = 0;
	let currency_id = 0;
	let apy = Perbill::from_percent(10);
	let alice: T::AccountId = get_account::<T>("ALICE");
	let nominator = Some(alice);

	assert_ok!(Fusion::<T>::create_pool(
		origin.clone().into(),
		pool_id,
		currency_id,
		apy,
		nominator
	));

	let valid_targets: BoundedVec<T::AccountId, T::MaxTargets> =
		vec![get_account::<T>("BOB")].try_into().unwrap();

	assert_ok!(Fusion::<T>::nominate(
		origin.clone().into(),
		pool_id,
		valid_targets
	));

	assert_ok!(Fusion::<T>::fill_pool_account(
		RawOrigin::Signed(get_account::<T>("ALICE")).into(),
		pool_id,
		1_000_000_000_000_000_000_000u128.saturated_into()
	));

	assert_ok!(Fusion::<T>::set_pool(
		origin.clone().into(),
		pool_id,
		None,
		Some(FusionPoolState::Open),
		ConfigOp::Noop,
		ConfigOp::Noop,
		None
	));
}

fn create_btc_currency<T: Config>() {
	let origin = RawOrigin::Root;
	let currency_id = 1;
	let nb_decimals = 8;
	let max_amount = 10_000_000_000;
	let min_amount = 1_000_000;
	let initial_conversion_rate: BalanceOf<T> = 10_000_000_000_000_000_000u128.saturated_into();
	let name: BoundedVec<u8, T::MaxCurrencyNameLength> = b"Bitcoin".to_vec().try_into().unwrap();

	assert_ok!(Fusion::<T>::create_currency(
		origin.into(),
		currency_id,
		name,
		nb_decimals,
		max_amount,
		min_amount,
		initial_conversion_rate,
	));
}

fn create_btc_pool<T: Config>() {
	let origin = RawOrigin::Root;
	let pool_id = 1;
	let currency_id = 1;
	let apy = Perbill::from_percent(5);
	let alice: T::AccountId = get_account::<T>("ALICE");
	let nominator = Some(alice);

	assert_ok!(Fusion::<T>::create_pool(
		origin.clone().into(),
		pool_id,
		currency_id,
		apy,
		nominator
	));

	let valid_targets: BoundedVec<T::AccountId, T::MaxTargets> =
		vec![get_account::<T>("BOB")].try_into().unwrap();

	assert_ok!(Fusion::<T>::nominate(
		origin.clone().into(),
		pool_id,
		valid_targets
	));

	assert_ok!(Fusion::<T>::fill_pool_account(
		RawOrigin::Signed(get_account::<T>("ALICE")).into(),
		pool_id,
		1_000_000_000_000_000_000_000u128.saturated_into()
	));

	assert_ok!(Fusion::<T>::set_pool(
		origin.clone().into(),
		pool_id,
		None,
		Some(FusionPoolState::Open),
		ConfigOp::Noop,
		ConfigOp::Noop,
		None
	));
}

fn fill_pool_with_dummy_members<T: Config>(pool_id: PoolId) {
	let mut pool = Fusion::<T>::pools(pool_id).unwrap();
	let max_members = T::MaxMembersPerPool::get();
	let dummy_address = FusionAddress::EvmAddress(H160::repeat_byte(0x09));
	let target_count = max_members / 2;
	for _ in 0..target_count {
		pool.members.try_push((dummy_address, 0)).unwrap();
	}
	Pools::<T>::insert(pool_id, pool);

	let pool = Fusion::<T>::pools(pool_id).unwrap();
	assert!(pool.members.len() > 0);
}

fn fill_era_data<T: Config>(
	era: EraIndex,
	validator: T::AccountId,
	fusion_address: FusionAddress,
	pool_id: PoolId,
	amount: u128,
) {
	EraDurations::<T>::insert(era, 120_000);

	let max_members = T::MaxMembersPerPool::get();
	let halfway_members = (max_members / 2) as usize;

	let mut exposure = FusionExposure::<T> {
		era,
		apy: Perbill::from_percent(10),
		total_avail: amount.saturated_into(),
		total_points: amount,
		user_points: BoundedVec::default(),
		targets: BoundedVec::try_from(vec![validator.clone()]).unwrap(),
		native_exposure_data: Some(
			BoundedVec::try_from(vec![(validator.clone(), amount.saturated_into())]).unwrap(),
		),
		boost_additional_apy: Perbill::from_percent(5),
		boost_members: Default::default(),
		boost_total_points: amount,
		boost_total_avail: amount.saturated_into(),
	};

	// Add dummy members
	for _ in 0..halfway_members {
		exposure
			.user_points
			.try_push((FusionAddress::EvmAddress(H160::repeat_byte(0x01)), 0))
			.unwrap();
		exposure
			.boost_members
			.try_push(FusionAddress::EvmAddress(H160::repeat_byte(0x01)))
			.unwrap();
	}

	// Add specified member
	exposure
		.user_points
		.try_push((fusion_address, amount))
		.unwrap();
	exposure.boost_members.try_push(fusion_address).unwrap();

	Exposures::<T>::insert(era, pool_id, exposure);

	T::StakingFusionDataProvider::add_dummy_era_points(validator, era);
}

fn create_era_reward<T: Config>(era: EraIndex, pool_id: PoolId, amount: u128) {
	let era_reward = EraReward::<T> {
		rewards: amount.saturated_into(),
		claimed_rewards: 0u128.saturated_into(),
		additional_rewards: amount.saturated_into(),
		additional_claimed_rewards: 0u128.saturated_into(),
	};

	EraRewards::<T>::insert(era, pool_id, era_reward);
}

#[benchmarks(
	where <T as Config>::RuntimeCall: From<frame_system::Call<T>>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create_currency() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();

		let origin = RawOrigin::Root;
		let currency_id = 0;
		let nb_decimals = 18;
		let max_amount = 1_000_000_000_000_000_000_000_000;
		let min_amount = 0;
		let initial_conversion_rate: BalanceOf<T> = 1_000_000_000_000_000_000u128.saturated_into();
		let name: BoundedVec<u8, T::MaxCurrencyNameLength> = b"Avail".to_vec().try_into().unwrap();

		#[extrinsic_call]
		_(
			origin,
			currency_id,
			name,
			nb_decimals,
			max_amount,
			min_amount,
			initial_conversion_rate,
		);

		Ok(())
	}

	#[benchmark]
	fn set_currency() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();

		let origin = RawOrigin::Root;
		let currency_id = 0;
		let max_amount = 1_000_000_000_000_000_000_000_000;
		let min_amount = 0;
		let name: BoundedVec<u8, T::MaxCurrencyNameLength> = b"Avail".to_vec().try_into().unwrap();

		#[extrinsic_call]
		_(
			origin,
			currency_id,
			Some(name),
			Some(max_amount),
			Some(min_amount),
		);

		Ok(())
	}

	#[benchmark]
	fn destroy_currency() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();

		let origin = RawOrigin::Root;
		let currency_id = 0;

		#[extrinsic_call]
		_(origin, currency_id);

		Ok(())
	}

	#[benchmark]
	fn set_currency_conversion_rate() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();

		let origin = RawOrigin::Root;
		let currency_id = 0;
		let new_conversion_rate: BalanceOf<T> = 1_100_000_000_000_000_000u128.saturated_into();

		#[extrinsic_call]
		_(origin, currency_id, new_conversion_rate);

		Ok(())
	}

	#[benchmark]
	fn create_pool() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();

		let origin = RawOrigin::Root;
		let pool_id = 0;
		let currency_id = 0;
		let apy = Perbill::from_percent(10);
		let alice: T::AccountId = get_account::<T>("ALICE");
		let nominator = Some(alice);

		#[extrinsic_call]
		_(origin, pool_id, currency_id, apy, nominator);

		Ok(())
	}

	#[benchmark]
	fn set_pool() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();

		let origin = RawOrigin::Root;
		let pool_id = 0;
		let apy = Some(Perbill::from_percent(15));
		let state = Some(FusionPoolState::Open);
		let nominator = ConfigOp::Set(get_account::<T>("DAVE"));
		let boost_data = ConfigOp::Set((Perbill::from_percent(5), 1_000_000_000_000_000_000));
		let retry_rewards_for_eras = None;

		#[extrinsic_call]
		_(
			origin,
			pool_id,
			apy,
			state,
			nominator,
			boost_data,
			retry_rewards_for_eras,
		);

		Ok(())
	}

	#[benchmark]
	fn set_pool_with_retry() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();
		create_btc_currency::<T>();
		create_btc_pool::<T>();

		let pool_id = 1;
		let apy = Some(Perbill::from_percent(15));
		let state = Some(FusionPoolState::Open);
		let nominator = ConfigOp::Set(get_account::<T>("DAVE"));
		let boost_data = ConfigOp::Set((Perbill::from_percent(5), 1_000_000_000_000_000_000));
		let retry_rewards_for_eras = Some(BoundedVec::try_from(vec![1]).unwrap());

		let fusion_address = FusionAddress::new_evm(H160::zero());
		fill_pool_with_dummy_members::<T>(1);
		fill_era_data::<T>(
			1,
			get_account::<T>("BOB"),
			fusion_address,
			1,
			1_000_000_000_000_000_000_000,
		);

		let origin = RawOrigin::Root;
		#[extrinsic_call]
		set_pool(
			origin,
			pool_id,
			apy,
			state,
			nominator,
			boost_data,
			retry_rewards_for_eras,
		);

		Ok(())
	}

	#[benchmark]
	fn destroy_pool() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();

		let origin = RawOrigin::Root;
		let pool_id = 0;
		let leftover_destination = Some(get_account::<T>("DAVE"));
		Fusion::<T>::destroy_pool(origin.clone().into(), pool_id, None).unwrap();

		#[extrinsic_call]
		_(origin, pool_id, leftover_destination);

		Ok(())
	}

	#[benchmark]
	fn fill_pool_account() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();

		let pool_id = 0;
		let amount: BalanceOf<T> = 100_000_000_000_000_000_000u128.saturated_into();
		let address = get_account::<T>("ALICE");
		let origin = RawOrigin::Signed(address);

		#[extrinsic_call]
		_(origin, pool_id, amount);

		Ok(())
	}

	#[benchmark]
	fn nominate() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();

		let origin = RawOrigin::Root;
		let pool_id = 0;
		let valid_targets: BoundedVec<T::AccountId, T::MaxTargets> =
			vec![get_account::<T>("BOB")].try_into().unwrap();

		#[extrinsic_call]
		_(origin, pool_id, valid_targets);

		Ok(())
	}

	#[benchmark]
	fn set_controller_address() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();

		let fusion_address = FusionAddress::new_evm(H160::zero());
		let controller_address: T::AccountId = get_account::<T>("ALICE");
		let new_controller_address: T::AccountId = get_account::<T>("BOB");
		Fusion::<T>::do_set_controller_address(fusion_address, Some(controller_address.clone()))
			.unwrap();
		let origin = RawOrigin::Signed(controller_address);

		#[extrinsic_call]
		_(origin, fusion_address, Some(new_controller_address));

		Ok(())
	}

	#[benchmark]
	fn set_slash_destination() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();

		let origin = RawOrigin::Root;
		let fusion_address = FusionAddress::new_evm(H160::zero());

		#[extrinsic_call]
		_(origin, Some(fusion_address), None);

		Ok(())
	}

	#[benchmark]
	fn update_max_tvl() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();

		let origin = RawOrigin::Root;

		#[extrinsic_call]
		_(
			origin,
			100_000_000_000_000_000_000_000_000u128.saturated_into(),
		);

		Ok(())
	}

	#[benchmark]
	fn set_compounding() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();
		create_btc_currency::<T>();
		create_btc_pool::<T>();

		let fusion_address = FusionAddress::new_evm(H160::zero());
		let controller_address: T::AccountId = get_account::<T>("ALICE");
		let origin = RawOrigin::Signed(controller_address.clone());
		Fusion::<T>::do_set_controller_address(fusion_address, Some(controller_address.clone()))
			.unwrap();
		Fusion::<T>::add_to_currency_balance(fusion_address, 1, 100_000_000, false).unwrap();
		Fusion::<T>::stake(origin.clone().into(), fusion_address, 1, 100_000_000).unwrap();

		#[extrinsic_call]
		_(origin, fusion_address, 1, false);

		Ok(())
	}

	#[benchmark]
	fn stake() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();
		create_btc_currency::<T>();
		create_btc_pool::<T>();

		let fusion_address = FusionAddress::new_evm(H160::zero());
		let controller_address: T::AccountId = get_account::<T>("ALICE");
		Fusion::<T>::do_set_controller_address(fusion_address, Some(controller_address.clone()))
			.unwrap();
		Fusion::<T>::add_to_currency_balance(fusion_address, 1, 100_000_000, false).unwrap();
		fill_pool_with_dummy_members::<T>(1);

		let origin = RawOrigin::Signed(controller_address);

		#[extrinsic_call]
		_(origin, fusion_address, 1, 100_000_000);

		Ok(())
	}

	#[benchmark]
	fn claim_rewards() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();
		create_btc_currency::<T>();
		create_btc_pool::<T>();

		let pool_id = 1;
		let fusion_address = FusionAddress::new_evm(H160::zero());
		fill_pool_with_dummy_members::<T>(1);
		fill_era_data::<T>(
			0,
			get_account::<T>("BOB"),
			fusion_address,
			1,
			1_000_000_000_000_000_000_000,
		);
		create_era_reward::<T>(0, 1, 1_000_000_000_000_000_000);

		let origin = RawOrigin::Signed(get_account::<T>("ALICE"));
		#[extrinsic_call]
		_(origin, 0, pool_id, fusion_address);

		Ok(())
	}

	#[benchmark]
	fn unbond_currency() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();
		create_btc_currency::<T>();
		create_btc_pool::<T>();

		let fusion_address = FusionAddress::new_evm(H160::zero());
		let controller_address: T::AccountId = get_account::<T>("ALICE");
		let origin = RawOrigin::Signed(controller_address.clone());
		Fusion::<T>::do_set_controller_address(fusion_address, Some(controller_address.clone()))
			.unwrap();
		Fusion::<T>::add_to_currency_balance(fusion_address, 1, 100_000_000, false).unwrap();
		Fusion::<T>::set_pool(
			RawOrigin::Root.into(),
			1,
			None,
			None,
			ConfigOp::Noop,
			ConfigOp::Set((Perbill::from_percent(5), 1_000_000_000_000_000_000)),
			None,
		)
		.unwrap();

		fill_pool_with_dummy_members::<T>(1);
		fill_era_data::<T>(
			0,
			get_account::<T>("BOB"),
			fusion_address,
			1,
			1_000_000_000_000_000_000_000,
		);

		Fusion::<T>::stake(origin.clone().into(), fusion_address, 1, 100_000_000).unwrap();
		HasBoost::<T>::insert(1, fusion_address, true);

		let max_members = T::MaxMembersPerPool::get();
		let halfway_members = (max_members / 2) as usize;
		let mut unbonding_chunk: BoundedVec<
			(FusionAddress, FusionCurrencyBalance),
			T::MaxMembersPerPool,
		> = BoundedVec::default();
		for _ in 0..halfway_members {
			unbonding_chunk
				.try_push((FusionAddress::EvmAddress(H160::repeat_byte(0x01)), 0))
				.unwrap()
		}
		UnbondingChunks::<T>::insert(1, T::BondingDuration::get(), unbonding_chunk);

		#[extrinsic_call]
		_(origin, fusion_address, 1, 100_000_000);

		Ok(())
	}

	#[benchmark]
	fn withdraw_unbonded_currency() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();
		create_btc_currency::<T>();
		create_btc_pool::<T>();

		let fusion_address = FusionAddress::new_evm(H160::zero());
		let controller_address: T::AccountId = get_account::<T>("ALICE");
		let origin = RawOrigin::Signed(controller_address.clone());
		Fusion::<T>::do_set_controller_address(fusion_address, Some(controller_address.clone()))
			.unwrap();
		Fusion::<T>::add_to_currency_balance(fusion_address, 1, 100_000_000, false).unwrap();
		Fusion::<T>::set_pool(
			RawOrigin::Root.into(),
			1,
			None,
			None,
			ConfigOp::Noop,
			ConfigOp::Set((Perbill::from_percent(5), 1_000_000_000_000_000_000)),
			None,
		)
		.unwrap();

		fill_pool_with_dummy_members::<T>(1);
		fill_era_data::<T>(
			0,
			get_account::<T>("BOB"),
			fusion_address,
			1,
			1_000_000_000_000_000_000_000,
		);

		Fusion::<T>::stake(origin.clone().into(), fusion_address, 1, 100_000_000).unwrap();
		HasBoost::<T>::insert(1, fusion_address, true);

		let max_members = T::MaxMembersPerPool::get();
		let halfway_members = (max_members / 2) as usize;
		let mut unbonding_chunk: BoundedVec<
			(FusionAddress, FusionCurrencyBalance),
			T::MaxMembersPerPool,
		> = BoundedVec::default();
		for _ in 0..halfway_members {
			unbonding_chunk
				.try_push((FusionAddress::EvmAddress(H160::repeat_byte(0x01)), 0))
				.unwrap()
		}
		unbonding_chunk
			.try_push((fusion_address, 100_000_000))
			.unwrap();
		UnbondingChunks::<T>::insert(1, T::BondingDuration::get(), unbonding_chunk);

		Fusion::<T>::unbond_currency(origin.clone().into(), fusion_address, 1, 100_000_000)
			.unwrap();

		T::StakingFusionDataProvider::set_dummy_active_era(T::BondingDuration::get() + 1);

		#[extrinsic_call]
		_(origin, fusion_address, 1);

		Ok(())
	}

	#[benchmark]
	fn withdraw_avail_to_controller() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();

		let fusion_address = FusionAddress::new_evm(H160::zero());
		let controller_address: T::AccountId = get_account::<T>("ALICE");
		let origin = RawOrigin::Signed(controller_address.clone());
		Fusion::<T>::do_set_controller_address(fusion_address, Some(controller_address.clone()))
			.unwrap();
		Fusion::<T>::add_to_currency_balance(fusion_address, 0, 10_000_000_000_000_000_000, false)
			.unwrap();

		#[extrinsic_call]
		_(origin, fusion_address);

		Ok(())
	}

	#[benchmark]
	fn set_pool_boost_allocations() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();
		create_btc_currency::<T>();
		create_btc_pool::<T>();

		let fusion_address = FusionAddress::new_evm(H160::zero());
		let controller_address: T::AccountId = get_account::<T>("ALICE");
		let origin = RawOrigin::Signed(controller_address.clone());
		Fusion::<T>::do_set_controller_address(fusion_address, Some(controller_address.clone()))
			.unwrap();

		Fusion::<T>::add_to_currency_balance(fusion_address, 0, 10_000_000_000_000_000_000, false)
			.unwrap();
		Fusion::<T>::add_to_currency_balance(fusion_address, 1, 100_000_000, false).unwrap();
		Fusion::<T>::set_pool(
			RawOrigin::Root.into(),
			0,
			None,
			None,
			ConfigOp::Noop,
			ConfigOp::Set((Perbill::from_percent(5), 1_000_000_000_000_000_000)),
			None,
		)
		.unwrap();
		Fusion::<T>::set_pool(
			RawOrigin::Root.into(),
			1,
			None,
			None,
			ConfigOp::Noop,
			ConfigOp::Set((Perbill::from_percent(5), 1_000_000_000_000_000_000)),
			None,
		)
		.unwrap();

		fill_pool_with_dummy_members::<T>(0);
		fill_pool_with_dummy_members::<T>(1);

		Fusion::<T>::stake(
			origin.clone().into(),
			fusion_address,
			0,
			10_000_000_000_000_000_000,
		)
		.unwrap();
		Fusion::<T>::stake(origin.clone().into(), fusion_address, 1, 100_000_000).unwrap();

		HasBoost::<T>::insert(0, fusion_address, true);
		HasBoost::<T>::insert(1, fusion_address, true);

		#[extrinsic_call]
		_(
			origin,
			fusion_address,
			BoundedVec::try_from(vec![0, 1]).unwrap(),
		);

		Ok(())
	}

	#[benchmark]
	fn withdraw_pool_account() -> Result<(), BenchmarkError> {
		init_benchmarks::<T>();
		create_avail_currency::<T>();
		create_avail_pool::<T>();

		let origin = RawOrigin::Root;

		#[extrinsic_call]
		_(
			origin,
			0,
			100_000_000_000_000_000_000u128.saturated_into(),
			get_account::<T>("ALICE"),
		);

		Ok(())
	}

	impl_benchmark_test_suite!(Fusion, crate::mock::new_test_ext(), crate::mock::Test);
}
