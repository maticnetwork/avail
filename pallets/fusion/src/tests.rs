use avail_core::currency::AVAIL;
use frame_support::{assert_noop, assert_ok, error::BadOrigin, BoundedVec};
use frame_system::RawOrigin;
use sp_core::H160;
use sp_runtime::TokenError;

use crate::{mock::*, *};

fn create_avail_currency() -> (types::FusionCurrency<mock::Test>, u128, u128) {
	let root: RuntimeOrigin = RawOrigin::Root.into();
	let avail_decimal: u8 = 18;
	let avail_max: u128 = 1_000_000 * AVAIL;
	let avail_min: u128 = 0;
	let avail_conversion_rate = AVAIL;
	let avail_name: BoundedVec<u8, MaxCurrencyNameLength> = b"Avail".to_vec().try_into().unwrap();
	assert_ok!(Fusion::create_currency(
		root,
		AVAIL_CURRENCY_ID,
		avail_name,
		avail_decimal,
		avail_max,
		avail_min,
		avail_conversion_rate
	));

	let fusion_currency = Currencies::<Test>::get(AVAIL_CURRENCY_ID).unwrap();

	let currency_rate = CurrencyRates::<Test>::get(
		MockStakingFusionDataProvider::active_era(),
		AVAIL_CURRENCY_ID,
	)
	.unwrap();
	let currency_rate_change = CurrencyRateChanges::<Test>::get(AVAIL_CURRENCY_ID);
	(fusion_currency, currency_rate, currency_rate_change)
}

fn create_avail_pool() -> types::FusionPool<mock::Test> {
	let root: RuntimeOrigin = RawOrigin::Root.into();
	let currency_id = AVAIL_CURRENCY_ID;
	let apy = Perbill::from_percent(5);
	let nominator: Option<u64> = Some(POOL_NOMINATOR_ROLE_ACCOUNT);

	assert_ok!(Fusion::create_pool(
		root.clone(),
		AVAIL_POOL_ID,
		currency_id,
		apy,
		nominator
	));

	let valid_targets: BoundedVec<u64, MaxTargets> =
		vec![VALIDATOR_1, VALIDATOR_2].try_into().unwrap();
	assert_ok!(Fusion::nominate(root.clone(), AVAIL_POOL_ID, valid_targets));

	assert_ok!(Fusion::fill_pool_account(
		RawOrigin::Signed(RANDOM_POT).into(),
		AVAIL_POOL_ID,
		1_000 * AVAIL
	));

	assert_ok!(Fusion::set_pool(
		root,
		AVAIL_POOL_ID,
		None,
		Some(FusionPoolState::Open),
		ConfigOp::Noop,
		ConfigOp::Noop,
		None
	));

	Pools::<Test>::get(AVAIL_POOL_ID).unwrap()
}

fn create_btc_currency() -> (types::FusionCurrency<mock::Test>, u128, u128) {
	let root: RuntimeOrigin = RawOrigin::Root.into();
	let btc_decimal: u8 = BTC_DECIMAL;
	let btc_max: u128 = 10_000_000_000; // 100 BTC
	let btc_min: u128 = 1_000_000; // 0.01 BTC
	let btc_conversion_rate = 10 * AVAIL; // 1 BTC = 10 Avail
	let btc_name: BoundedVec<u8, MaxCurrencyNameLength> = b"Bitcoin".to_vec().try_into().unwrap();
	assert_ok!(Fusion::create_currency(
		root,
		BTC_CURRENCY_ID,
		btc_name,
		btc_decimal,
		btc_max,
		btc_min,
		btc_conversion_rate
	));

	let fusion_currency = Currencies::<Test>::get(BTC_CURRENCY_ID).unwrap();

	let currency_rate =
		CurrencyRates::<Test>::get(MockStakingFusionDataProvider::active_era(), BTC_CURRENCY_ID)
			.unwrap();
	let currency_rate_change = CurrencyRateChanges::<Test>::get(BTC_CURRENCY_ID);
	(fusion_currency, currency_rate, currency_rate_change)
}

fn create_btc_pool() -> types::FusionPool<mock::Test> {
	let root: RuntimeOrigin = RawOrigin::Root.into();
	let currency_id = BTC_CURRENCY_ID;
	let apy = Perbill::from_percent(10);
	let nominator: Option<u64> = Some(POOL_NOMINATOR_ROLE_ACCOUNT);

	assert_ok!(Fusion::create_pool(
		root.clone(),
		BTC_POOL_ID,
		currency_id,
		apy,
		nominator
	));

	let valid_targets: BoundedVec<u64, MaxTargets> =
		vec![VALIDATOR_1, VALIDATOR_2].try_into().unwrap();
	assert_ok!(Fusion::nominate(root.clone(), BTC_POOL_ID, valid_targets));

	assert_ok!(Fusion::fill_pool_account(
		RawOrigin::Signed(RANDOM_POT).into(),
		BTC_POOL_ID,
		1_000 * AVAIL
	));

	assert_ok!(Fusion::set_pool(
		root,
		BTC_POOL_ID,
		None,
		Some(FusionPoolState::Open),
		ConfigOp::Noop,
		ConfigOp::Noop,
		None
	));

	Pools::<Test>::get(BTC_POOL_ID).unwrap()
}

mod create_currency {
	use super::*;

	#[test]
	fn create_currency() {
		new_test_ext().execute_with(|| {
			let (currency, currency_rate, _) = create_avail_currency();
			let event = RuntimeEvent::Fusion(Event::CurrencyCreated {
				currency_id: AVAIL_CURRENCY_ID,
				name: currency.name,
				nb_decimals: currency.nb_decimals,
				min_amount: currency.min_amount,
				max_amount: currency.max_amount,
				initial_conversion_rate: currency_rate,
			});
			System::assert_last_event(event);
		})
	}

	#[test]
	fn bad_origin() {
		new_test_ext().execute_with(|| {
			let non_root_origin: RuntimeOrigin = RawOrigin::Signed(ALICE).into();
			let avail_decimal: u8 = 18;
			let avail_max: u128 = 1_000_000 * AVAIL;
			let avail_min: u128 = 0;
			let avail_conversion_rate: u128 = AVAIL;
			let avail_name: BoundedVec<u8, MaxCurrencyNameLength> =
				b"Avail".to_vec().try_into().unwrap();

			assert_noop!(
				Fusion::create_currency(
					non_root_origin,
					AVAIL_CURRENCY_ID,
					avail_name,
					avail_decimal,
					avail_max,
					avail_min,
					avail_conversion_rate
				),
				BadOrigin
			);
		});
	}

	#[test]
	fn currency_already_exists() {
		new_test_ext().execute_with(|| {
			create_avail_currency();

			let root: RuntimeOrigin = RawOrigin::Root.into();
			let avail_decimal: u8 = 18;
			let avail_max: u128 = 1_000_000 * AVAIL;
			let avail_min: u128 = 0;
			let avail_conversion_rate = AVAIL;
			let avail_name: BoundedVec<u8, MaxCurrencyNameLength> =
				b"Avail".to_vec().try_into().unwrap();

			assert_noop!(
				Fusion::create_currency(
					root,
					AVAIL_CURRENCY_ID,
					avail_name,
					avail_decimal,
					avail_max,
					avail_min,
					avail_conversion_rate
				),
				Error::<Test>::CurrencyAlreadyExists
			);
		})
	}

	#[test]
	fn invalid_name() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let avail_decimal: u8 = 18;
			let avail_max: u128 = 1_000_000 * AVAIL;
			let avail_min: u128 = 0;
			let avail_conversion_rate = AVAIL;
			let invalid_name: BoundedVec<u8, MaxCurrencyNameLength> =
				b"".to_vec().try_into().unwrap();

			assert_noop!(
				Fusion::create_currency(
					root,
					AVAIL_CURRENCY_ID,
					invalid_name,
					avail_decimal,
					avail_max,
					avail_min,
					avail_conversion_rate
				),
				Error::<Test>::InvalidName
			);
		});
	}

	#[test]
	fn invalid_number_of_decimals() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let avail_max: u128 = 1_000_000 * AVAIL;
			let avail_min: u128 = 0;
			let avail_conversion_rate = AVAIL;
			let avail_name: BoundedVec<u8, MaxCurrencyNameLength> =
				b"Avail".to_vec().try_into().unwrap();

			assert_noop!(
				Fusion::create_currency(
					root,
					AVAIL_CURRENCY_ID,
					avail_name,
					0,
					avail_max,
					avail_min,
					avail_conversion_rate
				),
				Error::<Test>::InvalidNumberOfDecimals
			);
		});
	}

	#[test]
	fn invalid_max_number() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let avail_decimal: u8 = 18;
			let avail_max: u128 = 0;
			let avail_min: u128 = 1;
			let avail_conversion_rate = AVAIL;
			let avail_name: BoundedVec<u8, MaxCurrencyNameLength> =
				b"Avail".to_vec().try_into().unwrap();

			assert_noop!(
				Fusion::create_currency(
					root,
					AVAIL_CURRENCY_ID,
					avail_name,
					avail_decimal,
					avail_max,
					avail_min,
					avail_conversion_rate
				),
				Error::<Test>::InvalidMaxNumber
			);
		});
	}

	#[test]
	fn invalid_conversion_rate() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let avail_decimal: u8 = 18;
			let avail_max: u128 = 1_000_000 * AVAIL;
			let avail_min: u128 = 0;
			let invalid_conversion_rate: u128 = 0;
			let avail_name: BoundedVec<u8, MaxCurrencyNameLength> =
				b"Avail".to_vec().try_into().unwrap();

			assert_noop!(
				Fusion::create_currency(
					root,
					AVAIL_CURRENCY_ID,
					avail_name,
					avail_decimal,
					avail_max,
					avail_min,
					invalid_conversion_rate
				),
				Error::<Test>::InvalidConversionRate
			);
		});
	}

	#[test]
	fn no_min_amount_for_avail_currency() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let avail_decimal: u8 = 18;
			let avail_max: u128 = 1_000_000 * AVAIL;
			let avail_min: u128 = 1;
			let avail_conversion_rate: u128 = AVAIL;
			let avail_name: BoundedVec<u8, MaxCurrencyNameLength> =
				b"Avail".to_vec().try_into().unwrap();

			assert_noop!(
				Fusion::create_currency(
					root,
					AVAIL_CURRENCY_ID,
					avail_name,
					avail_decimal,
					avail_max,
					avail_min,
					avail_conversion_rate
				),
				Error::<Test>::NoMinAmountForAvailCurrency
			);
		});
	}
}

mod set_currency {
	use super::*;

	#[test]
	fn set_currency() {
		new_test_ext().execute_with(|| {
			let (mut currency, _, _) = create_avail_currency();
			let root: RuntimeOrigin = RawOrigin::Root.into();

			let new_name: Option<BoundedVec<u8, MaxCurrencyNameLength>> =
				Some(b"NewAvail".to_vec().try_into().unwrap());
			let new_max_amount: Option<u128> = Some(2_000_000 * AVAIL);
			let new_min_amount: Option<u128> = Some(0);

			assert_ok!(Fusion::set_currency(
				root,
				AVAIL_CURRENCY_ID,
				new_name.clone(),
				new_max_amount,
				new_min_amount
			));

			currency.name = new_name.clone().unwrap();
			currency.max_amount = new_max_amount.unwrap();
			currency.min_amount = new_min_amount.unwrap();

			let updated_currency = Currencies::<Test>::get(AVAIL_CURRENCY_ID).unwrap();
			assert_eq!(updated_currency.name, currency.name);
			assert_eq!(updated_currency.max_amount, currency.max_amount);
			assert_eq!(updated_currency.min_amount, currency.min_amount);
			assert_eq!(updated_currency.is_destroyed, currency.is_destroyed);

			System::assert_last_event(RuntimeEvent::Fusion(Event::CurrencySet {
				currency_id: AVAIL_CURRENCY_ID,
				name: new_name,
				min_amount: new_min_amount,
				max_amount: new_max_amount,
			}));
		});
	}

	#[test]
	fn bad_origin() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			let non_root_origin: RuntimeOrigin = RawOrigin::Signed(ALICE).into();
			let new_name: Option<BoundedVec<u8, MaxCurrencyNameLength>> =
				Some(b"NewAvail".to_vec().try_into().unwrap());
			let new_max_amount: Option<u128> = Some(2_000_000 * AVAIL);
			let new_min_amount: Option<u128> = Some(0);

			assert_noop!(
				Fusion::set_currency(
					non_root_origin,
					AVAIL_CURRENCY_ID,
					new_name,
					new_max_amount,
					new_min_amount
				),
				BadOrigin
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();

			let new_name: Option<BoundedVec<u8, MaxCurrencyNameLength>> =
				Some(b"NonExistent".to_vec().try_into().unwrap());
			let new_max_amount: Option<u128> = Some(2_000_000 * AVAIL);
			let new_min_amount: Option<u128> = Some(0);

			assert_noop!(
				Fusion::set_currency(root, INVALID_ID, new_name, new_max_amount, new_min_amount),
				Error::<Test>::CurrencyNotFound
			);
		});
	}

	#[test]
	fn currency_destroyed() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let (mut currency, _, _) = create_avail_currency();
			currency.is_destroyed = true;
			Currencies::<Test>::insert(AVAIL_CURRENCY_ID, currency);

			let new_name: Option<BoundedVec<u8, MaxCurrencyNameLength>> =
				Some(b"DestroyedAvail".to_vec().try_into().unwrap());
			let new_max_amount: Option<u128> = Some(2_000_000 * AVAIL);
			let new_min_amount: Option<u128> = Some(0);

			assert_noop!(
				Fusion::set_currency(
					root,
					AVAIL_CURRENCY_ID,
					new_name,
					new_max_amount,
					new_min_amount
				),
				Error::<Test>::CurrencyDestroyed
			);
		});
	}

	#[test]
	fn invalid_name() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			create_avail_currency();

			let invalid_name: Option<BoundedVec<u8, MaxCurrencyNameLength>> =
				Some(b"".to_vec().try_into().unwrap());
			let new_max_amount: Option<u128> = Some(2_000_000 * AVAIL);
			let new_min_amount: Option<u128> = Some(0);

			assert_noop!(
				Fusion::set_currency(
					root,
					AVAIL_CURRENCY_ID,
					invalid_name,
					new_max_amount,
					new_min_amount
				),
				Error::<Test>::InvalidName
			);
		});
	}

	#[test]
	fn invalid_max_number() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			create_avail_currency();

			let new_name: Option<BoundedVec<u8, MaxCurrencyNameLength>> =
				Some(b"Avail".to_vec().try_into().unwrap());
			let invalid_max_amount: Option<u128> = Some(0);
			let new_min_amount: Option<u128> = Some(0);

			assert_noop!(
				Fusion::set_currency(
					root,
					AVAIL_CURRENCY_ID,
					new_name,
					invalid_max_amount,
					new_min_amount
				),
				Error::<Test>::InvalidMaxNumber
			);
		});
	}

	#[test]
	fn invalid_max_amount() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let (mut currency, _, _) = create_avail_currency();
			currency.total_staked_native = 1_000_000;
			currency.total_unbonding_native = 500_000;
			Currencies::<Test>::insert(AVAIL_CURRENCY_ID, currency);

			let new_name: Option<BoundedVec<u8, MaxCurrencyNameLength>> =
				Some(b"Avail".to_vec().try_into().unwrap());
			let invalid_max_amount: Option<u128> = Some(1_000_000);
			let new_min_amount: Option<u128> = Some(0);

			assert_noop!(
				Fusion::set_currency(
					root,
					AVAIL_CURRENCY_ID,
					new_name,
					invalid_max_amount,
					new_min_amount
				),
				Error::<Test>::InvalidMaxAmount
			);
		});
	}

	#[test]
	fn invalid_min_amount() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			create_avail_currency();

			let new_name: Option<BoundedVec<u8, MaxCurrencyNameLength>> =
				Some(b"Avail".to_vec().try_into().unwrap());
			let new_max_amount: Option<u128> = Some(2_000_000 * AVAIL);
			let invalid_min_amount: Option<u128> = Some(3_000_000 * AVAIL);

			assert_noop!(
				Fusion::set_currency(
					root,
					AVAIL_CURRENCY_ID,
					new_name,
					new_max_amount,
					invalid_min_amount
				),
				Error::<Test>::InvalidMinAmount
			);
		});
	}

	#[test]
	fn no_min_amount_for_avail_currency() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			create_avail_currency();

			let new_name: Option<BoundedVec<u8, MaxCurrencyNameLength>> =
				Some(b"Avail".to_vec().try_into().unwrap());
			let new_max_amount: Option<u128> = Some(2_000_000 * AVAIL);
			let invalid_min_amount: Option<u128> = Some(1);

			assert_noop!(
				Fusion::set_currency(
					root,
					AVAIL_CURRENCY_ID,
					new_name,
					new_max_amount,
					invalid_min_amount
				),
				Error::<Test>::NoMinAmountForAvailCurrency
			);
		});
	}
}

mod destroy_currency {
	use super::*;

	#[test]
	fn destroy_currency() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			create_avail_currency();

			assert_ok!(Fusion::destroy_currency(root, AVAIL_CURRENCY_ID));

			let currency = Currencies::<Test>::get(AVAIL_CURRENCY_ID).unwrap();
			assert!(currency.is_destroyed);

			System::assert_last_event(RuntimeEvent::Fusion(Event::CurrencyDeleted {
				currency_id: AVAIL_CURRENCY_ID,
			}));
		});
	}

	#[test]
	fn bad_origin() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			let non_root_origin: RuntimeOrigin = RawOrigin::Signed(ALICE).into();

			assert_noop!(
				Fusion::destroy_currency(non_root_origin, AVAIL_CURRENCY_ID),
				BadOrigin
			);
		});
	}

	#[test]
	fn pool_exists_for_currency() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let root: RuntimeOrigin = RawOrigin::Root.into();

			assert_noop!(
				Fusion::destroy_currency(root, AVAIL_CURRENCY_ID),
				Error::<Test>::PoolExistsForCurrency
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();

			assert_noop!(
				Fusion::destroy_currency(root, INVALID_ID),
				Error::<Test>::CurrencyNotFound
			);
		});
	}

	#[test]
	fn currency_destroyed() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let (mut currency, _, _) = create_avail_currency();
			currency.is_destroyed = true;
			Currencies::<Test>::insert(AVAIL_CURRENCY_ID, currency);

			assert_noop!(
				Fusion::destroy_currency(root, AVAIL_CURRENCY_ID),
				Error::<Test>::CurrencyDestroyed
			);
		});
	}
}

mod set_currency_conversion_rate {
	use super::*;

	#[test]
	fn set_currency_conversion_rate() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			create_avail_currency();

			let new_conversion_rate: u128 = 2 * AVAIL;

			assert_ok!(Fusion::set_currency_conversion_rate(
				root,
				AVAIL_CURRENCY_ID,
				new_conversion_rate
			));

			let stored_conversion_rate = CurrencyRateChanges::<Test>::get(AVAIL_CURRENCY_ID);
			assert_eq!(stored_conversion_rate, new_conversion_rate);

			System::assert_last_event(RuntimeEvent::Fusion(Event::CurrencyConversionRateSet {
				currency_id: AVAIL_CURRENCY_ID,
				conversion_rate: new_conversion_rate,
			}));
		});
	}

	#[test]
	fn bad_origin() {
		new_test_ext().execute_with(|| {
			create_avail_currency();

			let non_root_origin: RuntimeOrigin = RawOrigin::Signed(ALICE).into();
			let new_conversion_rate: u128 = 2 * AVAIL;

			assert_noop!(
				Fusion::set_currency_conversion_rate(
					non_root_origin,
					AVAIL_CURRENCY_ID,
					new_conversion_rate
				),
				BadOrigin
			);
		});
	}

	#[test]
	fn invalid_conversion_rate() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			create_avail_currency();

			let invalid_conversion_rate: u128 = 0;

			assert_noop!(
				Fusion::set_currency_conversion_rate(
					root,
					AVAIL_CURRENCY_ID,
					invalid_conversion_rate
				),
				Error::<Test>::InvalidConversionRate
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let new_conversion_rate: u128 = 2 * AVAIL;

			assert_noop!(
				Fusion::set_currency_conversion_rate(root, INVALID_ID, new_conversion_rate),
				Error::<Test>::CurrencyNotFound
			);
		});
	}
	#[test]
	fn currency_destroyed() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let (mut currency, _, _) = create_avail_currency();
			currency.is_destroyed = true;
			Currencies::<Test>::insert(AVAIL_CURRENCY_ID, currency);

			let new_conversion_rate: u128 = 2 * AVAIL;

			assert_noop!(
				Fusion::set_currency_conversion_rate(root, AVAIL_CURRENCY_ID, new_conversion_rate),
				Error::<Test>::CurrencyDestroyed
			);
		});
	}
}

mod create_pool {
	use super::*;

	#[test]
	fn create_pool() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			let pool = create_avail_pool();

			assert_eq!(pool.currency_id, AVAIL_CURRENCY_ID);
			assert_eq!(pool.apy, Perbill::from_percent(5));
			assert_eq!(pool.nominator, Some(POOL_NOMINATOR_ROLE_ACCOUNT));
			assert_eq!(pool.state, FusionPoolState::Open);

			let pool_id_from_funds_account = PoolsAccountToId::<Test>::get(&pool.funds_account);
			assert_eq!(pool_id_from_funds_account, Some(AVAIL_POOL_ID));

			System::assert_has_event(RuntimeEvent::Fusion(Event::PoolCreated {
				pool_id: AVAIL_POOL_ID,
				currency_id: AVAIL_CURRENCY_ID,
				apy: Perbill::from_percent(5),
				state: FusionPoolState::Paused,
				nominator: Some(POOL_NOMINATOR_ROLE_ACCOUNT),
				funds_account: pool.funds_account,
				claimable_account: pool.claimable_account,
			}));
		});
	}

	#[test]
	fn bad_origin() {
		new_test_ext().execute_with(|| {
			create_avail_currency();

			let non_root_origin: RuntimeOrigin = RawOrigin::Signed(ALICE).into();
			let apy = Perbill::from_percent(5);
			let nominator: Option<u64> = Some(POOL_NOMINATOR_ROLE_ACCOUNT);

			assert_noop!(
				Fusion::create_pool(
					non_root_origin,
					AVAIL_POOL_ID,
					AVAIL_CURRENCY_ID,
					apy,
					nominator
				),
				BadOrigin
			);
		});
	}

	#[test]
	fn pool_already_exists() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let root: RuntimeOrigin = RawOrigin::Root.into();
			let apy = Perbill::from_percent(5);
			let nominator: Option<u64> = Some(POOL_NOMINATOR_ROLE_ACCOUNT);

			assert_noop!(
				Fusion::create_pool(root, AVAIL_POOL_ID, AVAIL_CURRENCY_ID, apy, nominator),
				Error::<Test>::PoolAlreadyExists
			);
		});
	}

	#[test]
	fn invalid_apy() {
		new_test_ext().execute_with(|| {
			create_avail_currency();

			let root: RuntimeOrigin = RawOrigin::Root.into();
			let apy = Perbill::zero();
			let nominator: Option<u64> = Some(POOL_NOMINATOR_ROLE_ACCOUNT);

			assert_noop!(
				Fusion::create_pool(root, AVAIL_POOL_ID, AVAIL_CURRENCY_ID, apy, nominator),
				Error::<Test>::InvalidAPY
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let apy = Perbill::from_percent(5);
			let nominator: Option<u64> = Some(POOL_NOMINATOR_ROLE_ACCOUNT);

			assert_noop!(
				Fusion::create_pool(root, AVAIL_POOL_ID, 9999, apy, nominator),
				Error::<Test>::CurrencyNotFound
			);
		});
	}

	#[test]
	fn currency_destroyed() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();
			let (mut currency, _, _) = create_avail_currency();
			currency.is_destroyed = true;
			Currencies::<Test>::insert(AVAIL_CURRENCY_ID, currency);

			let apy = Perbill::from_percent(5);
			let nominator: Option<u64> = Some(POOL_NOMINATOR_ROLE_ACCOUNT);

			assert_noop!(
				Fusion::create_pool(root, AVAIL_POOL_ID, AVAIL_CURRENCY_ID, apy, nominator),
				Error::<Test>::CurrencyDestroyed
			);
		});
	}
}

mod set_pool {
	use super::*;

	#[test]
	fn set_pool() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let root: RuntimeOrigin = RawOrigin::Root.into();
			let new_apy = Some(Perbill::from_percent(8));
			let new_state = Some(FusionPoolState::Paused);
			let new_nominator = ConfigOp::Set(ALICE);
			let new_boost_data_option = ConfigOp::Set((Perbill::from_percent(2), 1 * AVAIL));

			assert_ok!(Fusion::set_pool(
				root,
				AVAIL_POOL_ID,
				new_apy,
				new_state,
				new_nominator.clone(),
				new_boost_data_option.clone(),
				None
			));

			let pool = Pools::<Test>::get(0).unwrap();

			assert_eq!(pool.apy, new_apy.unwrap());
			assert_eq!(pool.state, new_state.unwrap());
			assert_eq!(pool.nominator, Some(ALICE));
			let new_boost_data = (Perbill::from_percent(2), 1 * AVAIL);
			let pool_boost_data = pool.boost_data.unwrap();
			assert_eq!(new_boost_data.0, pool_boost_data.additional_apy);
			assert_eq!(new_boost_data.1, pool_boost_data.min_avail_to_earn);

			System::assert_last_event(RuntimeEvent::Fusion(Event::PoolSet {
				pool_id: AVAIL_POOL_ID,
				apy: new_apy,
				state: new_state,
				nominator: new_nominator,
				boost_data: new_boost_data_option,
			}));
		});
	}

	#[test]
	fn set_pool_retry_rewards_for_eras() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			let pool = create_btc_pool();

			// We need to make the reward fail so we can try it again
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			// We set apy as something high
			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				Some(Perbill::from_percent(100)),
				None,
				ConfigOp::Noop,
				ConfigOp::Noop,
				None
			));

			// At end of era 3, reward for era 3 are going to get generated for fusion, so we go at the beginning of the era to make it fail
			run_to_era(3);

			// We now set the pool balance to zero
			assert_ok!(Balances::force_set_balance(
				RawOrigin::Root.into(),
				pool.funds_account,
				avail_core::currency::Balance::zero(),
			));

			// Now if we go to era 4, pool should be blocked
			run_to_era(4);

			// Check that we have the pool paused
			let missed_rewards = 38020000000000;
			System::assert_has_event(RuntimeEvent::Fusion(Event::RewardSet {
				era: 3,
				rewarded_pools: vec![],
				total_rewarded: 0,
				paused_pools: vec![1],
				paused_pools_missed_rewards: vec![missed_rewards],
				retry: false,
			}));

			// We can now retry what we want
			assert_ok!(Balances::force_set_balance(
				RawOrigin::Root.into(),
				pool.funds_account,
				10_000 * AVAIL,
			));

			// We retry generating reward for this era
			let retry_rewards_for_eras: BoundedVec<EraIndex, HistoryDepth> =
				vec![3].try_into().unwrap();
			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None,
				Some(FusionPoolState::Open), // Not mandatory to set pool to open to put rewards
				ConfigOp::Noop,
				ConfigOp::Noop,
				Some(retry_rewards_for_eras)
			));

			// We check that the pool was set and the reward too
			System::assert_has_event(RuntimeEvent::Fusion(Event::RewardSet {
				era: 3,
				rewarded_pools: vec![1],
				total_rewarded: missed_rewards,
				paused_pools: vec![],
				paused_pools_missed_rewards: vec![],
				retry: true,
			}));
			System::assert_has_event(RuntimeEvent::Fusion(Event::PoolSet {
				pool_id: 1,
				apy: None,
				state: Some(FusionPoolState::Open),
				nominator: ConfigOp::Noop,
				boost_data: ConfigOp::Noop,
			}));
		});
	}

	#[test]
	fn set_pool_pausing_pool_works() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			// We need to make the reward fail so we can try it again
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			// Run to some era so rewards get generated
			run_to_era(10);

			// Pause de pool
			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None,
				Some(FusionPoolState::Paused),
				ConfigOp::Noop,
				ConfigOp::Noop,
				None
			));

			// We should have rewards until era 10
			run_to_era(15);

			for era in 3..=15 {
				if era <= 10 {
					assert!(EraRewards::<Test>::get(era, BTC_POOL_ID).is_some());
				} else {
					assert!(EraRewards::<Test>::get(era, BTC_POOL_ID).is_none());
				}
			}

			// We unpause the pool
			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None,
				Some(FusionPoolState::Open),
				ConfigOp::Noop,
				ConfigOp::Noop,
				None
			));

			// Rewards should get generated again after 1 era, so era 17
			run_to_era(51);
			for era in 17..=50 {
				assert!(EraRewards::<Test>::get(era, BTC_POOL_ID).is_some());
			}
		});
	}

	#[test]
	fn bad_origin() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let non_root_origin: RuntimeOrigin = RawOrigin::Signed(ALICE).into();
			let state = Some(FusionPoolState::Open);

			assert_noop!(
				Fusion::set_pool(
					non_root_origin,
					AVAIL_POOL_ID,
					None,
					state,
					ConfigOp::Noop,
					ConfigOp::Noop,
					None
				),
				BadOrigin
			);
		});
	}

	#[test]
	fn pool_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();

			let root: RuntimeOrigin = RawOrigin::Root.into();

			let apy = Some(Perbill::from_percent(5));
			let state = Some(FusionPoolState::Open);
			let nominator: ConfigOp<u64> = ConfigOp::Set(POOL_NOMINATOR_ROLE_ACCOUNT);
			let boost_data: ConfigOp<(Perbill, u128)> = ConfigOp::Noop;

			assert_noop!(
				Fusion::set_pool(root, INVALID_ID, apy, state, nominator, boost_data, None),
				Error::<Test>::PoolNotFound
			);
		});
	}

	#[test]
	fn pool_is_destroying() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			let mut pool = create_avail_pool();

			pool.state = FusionPoolState::Destroying;
			Pools::<Test>::insert(AVAIL_POOL_ID, pool);

			let root: RuntimeOrigin = RawOrigin::Root.into();
			let apy = Some(Perbill::from_percent(5));
			let state = Some(FusionPoolState::Open);
			let nominator: ConfigOp<u64> = ConfigOp::Set(POOL_NOMINATOR_ROLE_ACCOUNT);
			let boost_data: ConfigOp<(Perbill, u128)> = ConfigOp::Noop;

			assert_noop!(
				Fusion::set_pool(root, AVAIL_POOL_ID, apy, state, nominator, boost_data, None),
				Error::<Test>::PoolIsDestroying
			);
		});
	}

	#[test]
	fn invalid_apy() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let root: RuntimeOrigin = RawOrigin::Root.into();
			let invalid_apy = Some(Perbill::zero());
			let state = Some(FusionPoolState::Open);
			let nominator: ConfigOp<u64> = ConfigOp::Set(POOL_NOMINATOR_ROLE_ACCOUNT);
			let boost_data: ConfigOp<(Perbill, u128)> = ConfigOp::Noop;

			assert_noop!(
				Fusion::set_pool(
					root,
					AVAIL_POOL_ID,
					invalid_apy,
					state,
					nominator,
					boost_data,
					None
				),
				Error::<Test>::InvalidAPY
			);
		});
	}

	#[test]
	fn cannot_set_pool_to_destroying() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let root: RuntimeOrigin = RawOrigin::Root.into();
			let apy = Some(Perbill::from_percent(5));
			let state = Some(FusionPoolState::Destroying);
			let nominator: ConfigOp<u64> = ConfigOp::Set(POOL_NOMINATOR_ROLE_ACCOUNT);
			let boost_data: ConfigOp<(Perbill, u128)> = ConfigOp::Noop;

			assert_noop!(
				Fusion::set_pool(root, AVAIL_POOL_ID, apy, state, nominator, boost_data, None),
				Error::<Test>::CannotSetPoolToDestroying
			);
		});
	}

	#[test]
	fn pool_is_not_nominating() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			let mut pool = create_avail_pool();

			pool.targets.clear();
			Pools::<Test>::insert(AVAIL_POOL_ID, pool);

			let root: RuntimeOrigin = RawOrigin::Root.into();
			let apy = Some(Perbill::from_percent(5));
			let state = Some(FusionPoolState::Open);
			let nominator: ConfigOp<u64> = ConfigOp::Set(POOL_NOMINATOR_ROLE_ACCOUNT);
			let boost_data: ConfigOp<(Perbill, u128)> = ConfigOp::Noop;

			assert_noop!(
				Fusion::set_pool(root, AVAIL_POOL_ID, apy, state, nominator, boost_data, None),
				Error::<Test>::PoolIsNotNominating
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			let root: RuntimeOrigin = RawOrigin::Root.into();

			let apy = Some(Perbill::from_percent(5));
			let state = Some(FusionPoolState::Open);
			let nominator: ConfigOp<u64> = ConfigOp::Set(POOL_NOMINATOR_ROLE_ACCOUNT);
			let boost_data: ConfigOp<(Perbill, u128)> = ConfigOp::Noop;

			Currencies::<Test>::remove(AVAIL_CURRENCY_ID);

			assert_noop!(
				Fusion::set_pool(root, AVAIL_POOL_ID, apy, state, nominator, boost_data, None),
				Error::<Test>::CurrencyNotFound
			);
		});
	}

	#[test]
	fn currency_destroyed() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			let mut currency = Currencies::<Test>::get(AVAIL_CURRENCY_ID).unwrap();

			currency.is_destroyed = true;
			Currencies::<Test>::insert(AVAIL_CURRENCY_ID, currency);

			let root: RuntimeOrigin = RawOrigin::Root.into();
			let apy = Some(Perbill::from_percent(5));
			let state = Some(FusionPoolState::Open);
			let nominator: ConfigOp<u64> = ConfigOp::Set(POOL_NOMINATOR_ROLE_ACCOUNT);
			let boost_data: ConfigOp<(Perbill, u128)> = ConfigOp::Noop;

			assert_noop!(
				Fusion::set_pool(root, AVAIL_POOL_ID, apy, state, nominator, boost_data, None),
				Error::<Test>::CurrencyDestroyed
			);
		});
	}

	#[test]
	fn setting_boost_work_as_intended() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());

			// No boost are set, check that if we set boost to none, nothing changes
			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None,
				None,
				ConfigOp::Noop,
				ConfigOp::Remove,
				None
			));

			assert!(Pools::<Test>::get(BTC_POOL_ID)
				.unwrap()
				.boost_data
				.is_none());
			assert!(!HasBoost::<Test>::get(BTC_POOL_ID, fusion_address));
			assert!(PoolsWithBoost::<Test>::get(BTC_POOL_ID).is_none());

			// No boost are set, we add a boost, see that data is correct, no user should have boost ofc
			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None,
				None,
				ConfigOp::Noop,
				ConfigOp::Set((Perbill::from_percent(2), 1 * AVAIL)),
				None
			));
			let boost_data = BoostData::<Test> {
				additional_apy: Perbill::from_percent(2),
				min_avail_to_earn: 1 * AVAIL,
				elligible_total_points: 0,
				elligible_members: BoundedVec::default(),
			};
			let new_pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			let new_pool_boost = new_pool.boost_data.unwrap();
			assert_eq!(boost_data.additional_apy, new_pool_boost.additional_apy);
			assert_eq!(
				boost_data.min_avail_to_earn,
				new_pool_boost.min_avail_to_earn
			);
			assert_eq!(
				boost_data.elligible_total_points,
				new_pool_boost.elligible_total_points
			);
			assert_eq!(
				boost_data.elligible_members,
				new_pool_boost.elligible_members
			);
			assert!(!HasBoost::<Test>::get(BTC_POOL_ID, fusion_address));
			assert_eq!(PoolsWithBoost::<Test>::get(BTC_POOL_ID).unwrap(), 1 * AVAIL);

			// Now we add a user to a pool with boost, we set the boost for the user and we set the boost to something else.
			// It should not change a thing as we cannot remove non-boost-elligible members on-chain
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));
			// Now we add 1 Avail in the user balance and we stake it
			Fusion::add_to_currency_balance(fusion_address, AVAIL_CURRENCY_ID, 1 * AVAIL, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				AVAIL_POOL_ID,
				1 * AVAIL
			));

			// Now the user join the pool boost
			assert_ok!(Fusion::set_pool_boost_allocations(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BoundedVec::try_from(vec![BTC_POOL_ID]).unwrap(),
			));
			// And we can change the boost
			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None,
				None,
				ConfigOp::Noop,
				ConfigOp::Set((Perbill::from_percent(2), 1_000_000 * AVAIL)),
				None
			));
			// And we check that user still has boost, cause we was part of it before
			let new_pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			let new_pool_boost = new_pool.boost_data.unwrap();
			assert_eq!(new_pool_boost.additional_apy, Perbill::from_percent(2));
			assert_eq!(
				new_pool_boost.elligible_total_points,
				1_000_000_000_000_000_000
			);
			assert_eq!(new_pool_boost.elligible_members, vec![fusion_address]);
			assert_eq!(new_pool_boost.min_avail_to_earn, 1_000_000 * AVAIL);
			assert!(HasBoost::<Test>::get(BTC_POOL_ID, fusion_address));
			assert_eq!(
				PoolsWithBoost::<Test>::get(BTC_POOL_ID).unwrap(),
				1_000_000 * AVAIL
			);

			// Now we can remove the boost and check that everything is fine
			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None,
				None,
				ConfigOp::Noop,
				ConfigOp::Remove,
				None
			));
			assert!(Pools::<Test>::get(BTC_POOL_ID)
				.unwrap()
				.boost_data
				.is_none());
			assert!(!HasBoost::<Test>::get(BTC_POOL_ID, fusion_address));
			assert!(PoolsWithBoost::<Test>::get(BTC_POOL_ID).is_none());
		})
	}
}

mod destroy_pool {
	use super::*;

	#[test]
	fn destroy_pool_set_to_destroying() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			assert_ok!(Fusion::destroy_pool(
				RawOrigin::Root.into(),
				AVAIL_POOL_ID,
				None
			));

			let pool = Pools::<Test>::get(AVAIL_POOL_ID).unwrap();
			assert_eq!(pool.state, FusionPoolState::Destroying);

			System::assert_last_event(RuntimeEvent::Fusion(Event::PoolDestroying {
				pool_id: AVAIL_POOL_ID,
			}));
		});
	}

	#[test]
	fn destroy_pool_actually_destroyed() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			// Before setting to destroying we will use a user to stake
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			// Let's progress to an advanced era so some rewards get generated
			run_to_era(5);

			// We set destroying to true
			assert_ok!(Fusion::destroy_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None
			));
			let pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			assert_eq!(pool.state, FusionPoolState::Destroying);

			// Now we do the cleanups before calling destroy again to actually delete it
			// We need to unbond, remove members, claim all rewards for the pool
			// Now that the pool is destroying, we can call permissionless extrinsic to clean
			assert_ok!(Fusion::unbond_currency_other(
				RawOrigin::Signed(POOL_NOMINATOR_ROLE_ACCOUNT).into(),
				fusion_address,
				BTC_POOL_ID
			));

			let era = Staking::active_era().unwrap().index;
			let unbonding_period = FusionBondingDuration::get();
			run_to_era(era + unbonding_period);

			let pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			assert_eq!(pool.state, FusionPoolState::Destroying);

			assert_ok!(Fusion::withdraw_unbonded_currency_other(
				RawOrigin::Signed(POOL_NOMINATOR_ROLE_ACCOUNT).into(),
				fusion_address,
				BTC_POOL_ID
			));

			//  Now that there is not user left, we need to claim all additional rewards
			let era = Staking::active_era().unwrap().index;
			let history_depth = HistoryDepth::get();
			let start_era = era.saturating_sub(history_depth);
			for era in start_era..era {
				if let Some(reward) = EraRewards::<Test>::get(era, BTC_POOL_ID) {
					if reward.rewards != reward.claimed_rewards
						|| reward.additional_rewards != reward.additional_claimed_rewards
					{
						// This means we have additional rewards to claim, we need to claim for each user
						if let Some(exposure) = Exposures::<Test>::get(era, BTC_POOL_ID) {
							for user_points in exposure.user_points {
								let fusion_address = user_points.0;
								let has_already_claimed_for_era =
									ClaimedRewards::<Test>::contains_key(
										era,
										(BTC_POOL_ID, fusion_address),
									);
								if !has_already_claimed_for_era {
									assert_ok!(Fusion::claim_rewards(
										RawOrigin::Signed(POOL_NOMINATOR_ROLE_ACCOUNT).into(),
										era,
										BTC_POOL_ID,
										fusion_address
									));
								}
							}
						}
					}
				}
			}

			// Now we can actually destroy the pool
			assert_ok!(Fusion::destroy_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				Some(RANDOM_POT)
			));

			assert!(Pools::<Test>::get(BTC_POOL_ID).is_none());
			System::assert_has_event(RuntimeEvent::Fusion(Event::PoolDeleted {
				pool_id: BTC_POOL_ID,
				leftover: 1000004677160839005421,
			}));
		});
	}

	#[test]
	fn bad_origin() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			assert_noop!(
				Fusion::destroy_pool(RawOrigin::Signed(ALICE).into(), AVAIL_POOL_ID, None),
				BadOrigin
			);
		});
	}

	#[test]
	fn destroy_pool_not_found() {
		new_test_ext().execute_with(|| {
			let root: RuntimeOrigin = RawOrigin::Root.into();

			assert_noop!(
				Fusion::destroy_pool(root, INVALID_ID, Some(RANDOM_POT)),
				Error::<Test>::PoolNotFound
			);
		});
	}

	#[test]
	fn pool_cannot_be_cleaned_due_to_members() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let root: RuntimeOrigin = RawOrigin::Root.into();

			assert_ok!(Fusion::destroy_pool(root.clone(), AVAIL_POOL_ID, None));

			let mut pool = Pools::<Test>::get(AVAIL_POOL_ID).unwrap();
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			pool.members.try_push((fusion_address, 1)).unwrap();
			Pools::<Test>::insert(AVAIL_POOL_ID, pool);

			assert_noop!(
				Fusion::destroy_pool(root, AVAIL_POOL_ID, Some(RANDOM_POT)),
				Error::<Test>::PoolCannotBeCleaned
			);
		});
	}

	#[test]
	fn pool_cannot_be_cleaned_due_to_unclaimed_rewards() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let root: RuntimeOrigin = RawOrigin::Root.into();
			assert_ok!(Fusion::destroy_pool(root.clone(), AVAIL_POOL_ID, None));
			run_to_era(2);

			EraRewards::<Test>::insert(
				1,
				AVAIL_POOL_ID,
				EraReward {
					rewards: 100,
					claimed_rewards: 50,
					additional_rewards: 50,
					additional_claimed_rewards: 25,
				},
			);

			assert_noop!(
				Fusion::destroy_pool(root, AVAIL_POOL_ID, Some(RANDOM_POT)),
				Error::<Test>::PoolCannotBeCleaned
			);
		});
	}

	#[test]
	fn no_leftover_destination_provided() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let root: RuntimeOrigin = RawOrigin::Root.into();

			assert_ok!(Fusion::destroy_pool(root.clone(), AVAIL_POOL_ID, None));

			let mut pool = Pools::<Test>::get(AVAIL_POOL_ID).unwrap();
			pool.total_staked_points = 0;
			pool.total_staked_native = 0;
			pool.total_unbonding_native = 0;
			pool.members.clear();
			Pools::<Test>::insert(AVAIL_POOL_ID, pool);

			assert_noop!(
				Fusion::destroy_pool(root, AVAIL_POOL_ID, None),
				Error::<Test>::NoLeftoverDestinationProvided
			);
		});
	}
}

mod fill_pool_account {
	use super::*;

	#[test]
	fn fill_pool_account() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let pool_funds_account = Fusion::get_pool_funds_account(AVAIL_POOL_ID);
			let initial_balance = Balances::free_balance(pool_funds_account);
			let amount = 1 * AVAIL;

			assert_ok!(Fusion::fill_pool_account(
				RawOrigin::Signed(ALICE).into(),
				AVAIL_POOL_ID,
				amount
			));

			let final_balance = Balances::free_balance(pool_funds_account);
			assert_eq!(final_balance, initial_balance + amount);

			System::assert_last_event(RuntimeEvent::Fusion(Event::FundsAccountFilled {
				sender: ALICE,
				amount,
			}));
		});
	}

	#[test]
	fn invalid_amount() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let sender = ALICE;
			let invalid_amount = 0;

			assert_noop!(
				Fusion::fill_pool_account(
					RawOrigin::Signed(sender).into(),
					AVAIL_POOL_ID,
					invalid_amount
				),
				Error::<Test>::InvalidAmount
			);
		});
	}

	#[test]
	fn insufficient_balance() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let sender = ALICE;
			let amount = 1_000 * AVAIL;

			// Ensure the sender has no balance
			assert_ok!(Balances::force_set_balance(
				RawOrigin::Root.into(),
				sender,
				1 * AVAIL,
			));
			assert_eq!(Balances::free_balance(sender), 1 * AVAIL);

			assert_noop!(
				Fusion::fill_pool_account(RawOrigin::Signed(sender).into(), AVAIL_POOL_ID, amount),
				TokenError::FundsUnavailable
			);
		});
	}
}

mod nominate {
	use super::*;

	#[test]
	fn nominate_from_root() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let root = RawOrigin::Root.into();
			let valid_targets: BoundedVec<u64, MaxTargets> =
				vec![VALIDATOR_1, VALIDATOR_2].try_into().unwrap();

			assert_ok!(Fusion::nominate(root, AVAIL_POOL_ID, valid_targets.clone()));

			let pool = Pools::<Test>::get(AVAIL_POOL_ID).unwrap();
			assert_eq!(pool.targets, valid_targets);

			System::assert_last_event(RuntimeEvent::Fusion(Event::Nominated {
				pool_id: AVAIL_POOL_ID,
				targets: valid_targets,
			}));
		});
	}

	#[test]
	fn nominate_from_nominator() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let nominator = RawOrigin::Signed(POOL_NOMINATOR_ROLE_ACCOUNT).into();
			let valid_targets: BoundedVec<u64, MaxTargets> =
				vec![VALIDATOR_1, VALIDATOR_2].try_into().unwrap();

			assert_ok!(Fusion::nominate(
				nominator,
				AVAIL_POOL_ID,
				valid_targets.clone()
			));

			let pool = Pools::<Test>::get(AVAIL_POOL_ID).unwrap();
			assert_eq!(pool.targets, valid_targets);

			System::assert_last_event(RuntimeEvent::Fusion(Event::Nominated {
				pool_id: AVAIL_POOL_ID,
				targets: valid_targets,
			}));
		});
	}

	#[test]
	fn bad_origin() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let no_origin = RawOrigin::None.into();
			let valid_targets: BoundedVec<u64, MaxTargets> =
				vec![VALIDATOR_1, VALIDATOR_2].try_into().unwrap();

			assert_noop!(
				Fusion::nominate(no_origin, AVAIL_POOL_ID, valid_targets),
				BadOrigin
			);
		});
	}

	#[test]
	fn pool_not_found() {
		new_test_ext().execute_with(|| {
			let root = RawOrigin::Root.into();
			let valid_targets: BoundedVec<u64, MaxTargets> =
				vec![VALIDATOR_1, VALIDATOR_2].try_into().unwrap();

			assert_noop!(
				Fusion::nominate(root, INVALID_ID, valid_targets),
				Error::<Test>::PoolNotFound
			);
		});
	}

	#[test]
	fn not_authorized() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let nominator = RawOrigin::Signed(ALICE).into();
			let valid_targets: BoundedVec<u64, MaxTargets> =
				vec![VALIDATOR_1, VALIDATOR_2].try_into().unwrap();

			assert_noop!(
				Fusion::nominate(nominator, AVAIL_POOL_ID, valid_targets),
				Error::<Test>::NotAuthorized
			);
		});
	}

	#[test]
	fn pool_is_destroying() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let valid_targets: BoundedVec<u64, MaxTargets> =
				vec![VALIDATOR_1, VALIDATOR_2].try_into().unwrap();

			assert_ok!(Fusion::destroy_pool(
				RawOrigin::Root.into(),
				AVAIL_POOL_ID,
				None
			));

			assert_noop!(
				Fusion::nominate(RawOrigin::Root.into(), AVAIL_POOL_ID, valid_targets),
				Error::<Test>::PoolIsDestroying
			);
		});
	}

	#[test]
	fn active_pool_needs_targets() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let empty_targets: BoundedVec<u64, MaxTargets> = vec![].try_into().unwrap();

			assert_noop!(
				Fusion::nominate(RawOrigin::Root.into(), AVAIL_POOL_ID, empty_targets),
				Error::<Test>::ActivePoolNeedsTargets
			);
		});
	}

	#[test]
	fn no_valid_validators() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let root = RawOrigin::Root.into();
			let invalid_targets: BoundedVec<u64, MaxTargets> = vec![ALICE].try_into().unwrap();

			assert_noop!(
				Fusion::nominate(root, AVAIL_POOL_ID, invalid_targets),
				Error::<Test>::NoValidValidators
			);
		});
	}
}

// TODO Commented as the extrinsic is mocked for test
mod set_controller_address {
	// 	use super::*;

	// 	#[test]
	// 	fn set_controller_from_controller() {
	// 		new_test_ext().execute_with(|| {
	// 			let fusion_address = FusionAddress::EvmAddress(H160::zero());
	// 			let old_controller = FUSION_STAKER;
	// 			let new_controller = ALICE;

	// 			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, old_controller);

	// 			assert_eq!(
	// 				FusionAddressToSubstrateAddress::<Test>::get(fusion_address),
	// 				Some(old_controller)
	// 			);

	// 			assert_ok!(Fusion::set_controller_address(
	// 				RawOrigin::Signed(old_controller).into(),
	// 				fusion_address,
	// 				Some(new_controller)
	// 			));

	// 			assert_eq!(
	// 				FusionAddressToSubstrateAddress::<Test>::get(fusion_address),
	// 				Some(new_controller)
	// 			);

	// 			System::assert_last_event(RuntimeEvent::Fusion(Event::ControllerAddressSet {
	// 				fusion_address,
	// 				new_controller_address: Some(new_controller),
	// 			}));
	// 		});
	// 	}

	// 	#[test]
	// 	fn set_controller_from_root() {
	// 		new_test_ext().execute_with(|| {
	// 			let fusion_address = FusionAddress::EvmAddress(H160::zero());
	// 			let old_controller = FUSION_STAKER;

	// 			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, old_controller);

	// 			assert_eq!(
	// 				FusionAddressToSubstrateAddress::<Test>::get(fusion_address),
	// 				Some(old_controller)
	// 			);

	// 			assert_ok!(Fusion::set_controller_address(
	// 				RawOrigin::Root.into(),
	// 				fusion_address,
	// 				None
	// 			));

	// 			assert_eq!(
	// 				FusionAddressToSubstrateAddress::<Test>::get(fusion_address),
	// 				None
	// 			);

	// 			System::assert_last_event(RuntimeEvent::Fusion(Event::ControllerAddressSet {
	// 				fusion_address,
	// 				new_controller_address: None,
	// 			}));
	// 		});
	// 	}

	// 	#[test]
	// 	fn bad_origin() {
	// 		new_test_ext().execute_with(|| {
	// 			create_avail_currency();
	// 			create_avail_pool();

	// 			let no_origin = RawOrigin::None.into();
	// 			let fusion_address = FusionAddress::EvmAddress(H160::zero());

	// 			assert_noop!(
	// 				Fusion::set_controller_address(no_origin, fusion_address, None),
	// 				BadOrigin
	// 			);
	// 		});
	// 	}

	// 	#[test]
	// 	fn invalid_substrate_address_not_mapped() {
	// 		new_test_ext().execute_with(|| {
	// 			let fusion_address = FusionAddress::EvmAddress(H160::zero());
	// 			let sender = FUSION_STAKER;

	// 			assert_noop!(
	// 				Fusion::set_controller_address(
	// 					RawOrigin::Signed(sender).into(),
	// 					fusion_address,
	// 					Some(ALICE)
	// 				),
	// 				Error::<Test>::InvalidSubstrateAddress
	// 			);
	// 		});
	// 	}

	// 	#[test]
	// 	fn invalid_substrate_address_incorrect_controller() {
	// 		new_test_ext().execute_with(|| {
	// 			let fusion_address = FusionAddress::EvmAddress(H160::zero());
	// 			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, ALICE);
	// 			let sender = FUSION_STAKER;

	// 			assert_noop!(
	// 				Fusion::set_controller_address(
	// 					RawOrigin::Signed(sender).into(),
	// 					fusion_address,
	// 					Some(ALICE)
	// 				),
	// 				Error::<Test>::InvalidSubstrateAddress
	// 			);
	// 		});
	// 	}

	// 	#[test]
	// 	fn root_can_only_remove_controller() {
	// 		new_test_ext().execute_with(|| {
	// 			let fusion_address = FusionAddress::EvmAddress(H160::zero());
	// 			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, FUSION_STAKER);

	// 			assert_noop!(
	// 				Fusion::set_controller_address(RawOrigin::Root.into(), fusion_address, Some(ALICE)),
	// 				Error::<Test>::RootCanOnlyRemoveController
	// 			);
	// 		});
	// 	}

	// 	#[test]
	// 	fn cannot_set_controller_for_slash_destination() {
	// 		new_test_ext().execute_with(|| {
	// 			let slash_destination = FusionAddress::EvmAddress(H160::repeat_byte(0x01));

	// 			assert_noop!(
	// 				Fusion::set_controller_address(
	// 					RawOrigin::Signed(SLASH_DESTINATION).into(),
	// 					slash_destination,
	// 					Some(ALICE)
	// 				),
	// 				Error::<Test>::CannotSetControllerForSlashDestination
	// 			);
	// 		});
	// 	}
}

mod set_slash_destination {
	use super::*;

	#[test]
	fn set_slash_destination() {
		new_test_ext().execute_with(|| {
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = ALICE;

			assert_ok!(Fusion::set_slash_destination(
				RawOrigin::Root.into(),
				Some(fusion_address),
				Some(controller_address)
			));

			assert_eq!(SlashDestination::<Test>::get(), Some(fusion_address));
			assert_eq!(
				FusionAddressToSubstrateAddress::<Test>::get(fusion_address),
				Some(controller_address)
			);

			System::assert_last_event(RuntimeEvent::Fusion(Event::SlashDestinationSet {
				fusion_address: Some(fusion_address),
				controller_address: Some(controller_address),
			}));

			assert_ok!(Fusion::set_slash_destination(
				RawOrigin::Root.into(),
				None,
				None
			));

			assert_eq!(SlashDestination::<Test>::get(), None);
			assert_eq!(
				FusionAddressToSubstrateAddress::<Test>::get(fusion_address),
				None
			);

			System::assert_last_event(RuntimeEvent::Fusion(Event::SlashDestinationSet {
				fusion_address: None,
				controller_address: None,
			}));
		});
	}

	#[test]
	fn bad_origin() {
		new_test_ext().execute_with(|| {
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = ALICE;

			assert_noop!(
				Fusion::set_slash_destination(
					RawOrigin::Signed(ALICE).into(),
					Some(fusion_address),
					Some(controller_address)
				),
				BadOrigin
			);
		});
	}
}

mod update_max_tvl {
	use super::*;

	#[test]
	fn update_max_tvl() {
		new_test_ext().execute_with(|| {
			let root = RawOrigin::Root.into();
			let initial_tvl = 5_000 * AVAIL;
			let new_max_tvl = 10_000 * AVAIL;

			TotalValueLockedData::<Test>::put(TVLData {
				total_value_locked: initial_tvl,
				max_total_value_locked: initial_tvl,
			});

			assert_ok!(Fusion::update_max_tvl(root, new_max_tvl));

			let tvl_data = TotalValueLockedData::<Test>::get();
			assert_eq!(tvl_data.max_total_value_locked, new_max_tvl);
			assert_eq!(tvl_data.total_value_locked, initial_tvl);

			System::assert_last_event(RuntimeEvent::Fusion(Event::MaxTVLUpdated(new_max_tvl)));
		});
	}

	#[test]
	fn bad_origin() {
		new_test_ext().execute_with(|| {
			let non_root_origin = RawOrigin::Signed(ALICE).into();
			let new_max_tvl = 10_000 * AVAIL;

			assert_noop!(
				Fusion::update_max_tvl(non_root_origin, new_max_tvl),
				BadOrigin
			);
		});
	}

	#[test]
	fn max_tvl_reached() {
		new_test_ext().execute_with(|| {
			let root = RawOrigin::Root.into();
			let initial_tvl = 5_000 * AVAIL;
			let new_max_tvl = 4_000 * AVAIL;

			TotalValueLockedData::<Test>::put(TVLData {
				total_value_locked: initial_tvl,
				max_total_value_locked: initial_tvl,
			});

			assert_noop!(
				Fusion::update_max_tvl(root, new_max_tvl),
				Error::<Test>::MaxTVLReached
			);
		});
	}
}

mod set_compounding {
	use super::*;

	#[test]
	fn set_compounding() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));
			assert_ok!(Fusion::set_compounding(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				false,
			));

			assert!(
				!Memberships::<Test>::get(fusion_address, BTC_POOL_ID)
					.unwrap()
					.is_compounding,
			);

			System::assert_last_event(RuntimeEvent::Fusion(Event::CompoundingSet {
				fusion_address,
				pool_id: BTC_POOL_ID,
				compound: false,
			}));

			assert_ok!(Fusion::set_compounding(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				true,
			));

			assert!(
				Memberships::<Test>::get(fusion_address, BTC_POOL_ID)
					.unwrap()
					.is_compounding,
			);

			System::assert_last_event(RuntimeEvent::Fusion(Event::CompoundingSet {
				fusion_address,
				pool_id: BTC_POOL_ID,
				compound: true,
			}));
		});
	}

	#[test]
	fn invalid_substrate_address() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));
			FusionAddressToSubstrateAddress::<Test>::remove(fusion_address);
			assert_noop!(
				Fusion::set_compounding(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					false,
				),
				Error::<Test>::InvalidSubstrateAddress
			);
		});
	}

	#[test]
	fn membership_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			assert_noop!(
				Fusion::set_compounding(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					false,
				),
				Error::<Test>::MembershipNotFound
			);
		});
	}

	#[test]
	fn pool_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			Pools::<Test>::remove(BTC_POOL_ID);

			assert_noop!(
				Fusion::set_compounding(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					false,
				),
				Error::<Test>::PoolNotFound
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			Currencies::<Test>::remove(BTC_CURRENCY_ID);

			assert_noop!(
				Fusion::set_compounding(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					false,
				),
				Error::<Test>::CurrencyNotFound
			);
		});
	}

	#[test]
	fn cannot_set_compounding_with_less_than_minimum() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			let mut membership = Memberships::<Test>::get(fusion_address, BTC_POOL_ID).unwrap();
			membership.active_points = 0;
			Memberships::<Test>::insert(fusion_address, BTC_POOL_ID, membership);

			assert_ok!(Fusion::set_compounding(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				false,
			));
		});
	}
}

mod stake {
	use super::*;

	#[test]
	fn stake() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 200_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			System::assert_last_event(RuntimeEvent::Fusion(Event::PoolJoined {
				fusion_address,
				pool_id: BTC_POOL_ID,
				currency_id: BTC_POOL_ID,
				amount: 100_000_000,
				points: 1_000_000_000_000_000_000,
			}));

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			System::assert_last_event(RuntimeEvent::Fusion(Event::PoolBondExtra {
				fusion_address,
				pool_id: BTC_POOL_ID,
				currency_id: BTC_POOL_ID,
				amount: 100_000_000,
				points: 1_000_000_000_000_000_000,
			}));
		});
	}

	#[test]
	fn invalid_substrate_address() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					100_000_000
				),
				Error::<Test>::InvalidSubstrateAddress
			);
		});
	}

	#[test]
	fn pool_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					INVALID_ID,
					100_000_000
				),
				Error::<Test>::PoolNotFound
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			Currencies::<Test>::remove(BTC_CURRENCY_ID);

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					100_000_000
				),
				Error::<Test>::CurrencyNotFound
			);
		});
	}

	#[test]
	fn invalid_amount() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					0
				),
				Error::<Test>::InvalidAmount
			);
		});
	}

	#[test]
	fn pool_not_open() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			let mut pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			pool.state = FusionPoolState::Paused;
			Pools::<Test>::insert(BTC_POOL_ID, pool);

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					10_000_000
				),
				Error::<Test>::PoolNotOpen
			);

			let mut pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			pool.state = FusionPoolState::Blocked;
			Pools::<Test>::insert(BTC_POOL_ID, pool);

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					10_000_000
				),
				Error::<Test>::PoolNotOpen
			);

			let mut pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			pool.state = FusionPoolState::Destroying;
			Pools::<Test>::insert(BTC_POOL_ID, pool);

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					10_000_000
				),
				Error::<Test>::PoolNotOpen
			);
		});
	}

	#[test]
	fn currency_destroyed() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			let mut currency = Currencies::<Test>::get(BTC_CURRENCY_ID).unwrap();
			currency.is_destroyed = true;
			Currencies::<Test>::insert(BTC_CURRENCY_ID, currency);

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					10_000_000
				),
				Error::<Test>::CurrencyDestroyed
			);
		});
	}

	#[test]
	fn bond_would_exceed_max_for_currency() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(
				fusion_address,
				BTC_CURRENCY_ID,
				100_000_000_000,
				false,
			)
			.unwrap();

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					100_000_000_000
				),
				Error::<Test>::BondWouldExceedMaxForCurrency
			);
		});
	}

	#[test]
	fn no_currency_balance_for_user() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					10_000_000
				),
				Error::<Test>::NoCurrencyBalanceForUser
			);
		});
	}

	#[test]
	fn not_enough_currency_balance_for_user() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 10_000_000, false)
				.unwrap();

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					10_000_001
				),
				Error::<Test>::NotEnoughCurrencyBalanceForUser
			);
		});
	}

	#[test]
	fn bond_amount_too_low() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000, false)
				.unwrap();

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					100_000
				),
				Error::<Test>::BondAmoundTooLow
			);
		});
	}

	#[test]
	fn pool_membership_limit_reached() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let random_address = FusionAddress::EvmAddress(H160::repeat_byte(0x01));
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 10_000_000, false)
				.unwrap();

			let mut pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			for _ in 0..MaxMembersPerPool::get() {
				pool.members.try_push((random_address, 0)).unwrap();
			}
			Pools::<Test>::insert(BTC_POOL_ID, pool);

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					10_000_000
				),
				Error::<Test>::PoolMemberLimitReached
			);
		});
	}

	#[test]
	fn bond_extra_bond_amount_too_low() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 200_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			System::assert_last_event(RuntimeEvent::Fusion(Event::PoolJoined {
				fusion_address,
				pool_id: BTC_POOL_ID,
				currency_id: BTC_POOL_ID,
				amount: 100_000_000,
				points: 1_000_000_000_000_000_000,
			}));

			let mut membership = Memberships::<Test>::get(fusion_address, BTC_POOL_ID).unwrap();
			membership.active_points = 0;
			Memberships::<Test>::insert(fusion_address, BTC_POOL_ID, membership);

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					1
				),
				Error::<Test>::BondAmoundTooLow
			);
		});
	}

	#[test]
	fn max_tvl_reached() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(
				fusion_address,
				BTC_CURRENCY_ID,
				1_000_000_000_000_000,
				false,
			)
			.unwrap();

			assert_ok!(Fusion::set_currency(
				RawOrigin::Root.into(),
				BTC_CURRENCY_ID,
				None,
				Some(1_000_000_000_000_000),
				None
			));

			assert_noop!(
				Fusion::stake(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					1_000_000_000_000_000
				),
				Error::<Test>::MaxTVLReached
			);
		});
	}
}

mod claim_rewards {
	use super::*;

	#[test]
	fn claim_rewards() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			let pool = create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			let era_to_run = 4;
			run_to_era(era_to_run);
			let era_to_reward = era_to_run - 1;

			assert_ok!(Fusion::claim_rewards(
				RawOrigin::Signed(controller_address).into(),
				era_to_reward,
				BTC_POOL_ID,
				fusion_address,
			));

			let reward = 3802000000000;

			let era_duration: u128 = EraDurations::<Test>::get(era_to_reward).unwrap().into();
			let user_points = Memberships::<Test>::get(fusion_address, BTC_POOL_ID)
				.unwrap()
				.active_points;
			let staked_amount_in_avail: u128 = pool
				.points_to_avail(user_points, BTC_POOL_ID, None, None)
				.unwrap();
			let milliseconds_per_year: u128 = 1000 * 3600 * 24 * 36525 / 100;

			let expected_reward_for_a_year = pool.apy * staked_amount_in_avail;
			let fraction_of_year = Perbill::from_rational(era_duration, milliseconds_per_year);
			let expected_reward = fraction_of_year * expected_reward_for_a_year;

			// When we claim, it also compounds the reward amount to the avail pool
			System::assert_has_event(RuntimeEvent::Fusion(Event::PoolJoined {
				fusion_address,
				currency_id: AVAIL_CURRENCY_ID,
				pool_id: AVAIL_POOL_ID,
				amount: reward,
				points: reward,
			}));

			System::assert_last_event(RuntimeEvent::Fusion(Event::RewardClaimed {
				fusion_address,
				pool_id: BTC_POOL_ID,
				era: era_to_reward,
				reward,
			}));

			assert_eq!(reward, expected_reward);

			// Now let's add a boost to the pool, join the boost and claim boosted rewards
			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None,
				None,
				ConfigOp::Noop,
				ConfigOp::Set((Perbill::from_percent(10), 0)),
				None
			));
			assert_ok!(Fusion::set_pool_boost_allocations(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BoundedVec::try_from(vec![BTC_POOL_ID]).unwrap()
			));

			let era_to_run = era_to_run + 3; // Boost will take effect in reward 2 eras later so 3 to get rewards
			run_to_era(era_to_run);
			let era_to_reward = era_to_run - 1;

			// We now claim boosted rewards
			assert_ok!(Fusion::claim_rewards(
				RawOrigin::Signed(controller_address).into(),
				era_to_reward,
				BTC_POOL_ID,
				fusion_address,
			));

			let reward = 7604000000000;
			let pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			let era_duration: u128 = EraDurations::<Test>::get(era_to_reward).unwrap().into();
			let user_points = Memberships::<Test>::get(fusion_address, BTC_POOL_ID)
				.unwrap()
				.active_points;
			let staked_amount_in_avail: u128 = pool
				.points_to_avail(user_points, BTC_POOL_ID, None, None)
				.unwrap();
			let milliseconds_per_year: u128 = 1000 * 3600 * 24 * 36525 / 100;

			let expected_reward_for_a_year =
				(pool.apy + pool.boost_data.unwrap().additional_apy) * staked_amount_in_avail;
			let fraction_of_year = Perbill::from_rational(era_duration, milliseconds_per_year);
			let expected_reward = fraction_of_year * expected_reward_for_a_year;

			// When we claim, it also compounds the reward amount to the avail pool, now we should add extra
			System::assert_has_event(RuntimeEvent::Fusion(Event::PoolBondExtra {
				fusion_address,
				currency_id: AVAIL_CURRENCY_ID,
				pool_id: AVAIL_POOL_ID,
				amount: reward,
				points: reward,
			}));

			System::assert_last_event(RuntimeEvent::Fusion(Event::RewardClaimed {
				fusion_address,
				pool_id: BTC_POOL_ID,
				era: era_to_reward,
				reward,
			}));

			assert_eq!(reward, expected_reward);
		});
	}

	#[test]
	fn claim_rewards_clears_era_rewards() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			let fusion_address2 = FusionAddress::EvmAddress(H160::repeat_byte(0x01));
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address2, controller_address);
			Fusion::add_to_currency_balance(fusion_address2, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address2,
				BTC_POOL_ID,
				staked_amount
			));

			let era_to_run = 4;
			run_to_era(era_to_run);
			let era_to_reward = era_to_run - 1;

			assert_ok!(Fusion::claim_rewards(
				RawOrigin::Signed(controller_address).into(),
				era_to_reward,
				BTC_POOL_ID,
				fusion_address,
			));
			assert_ok!(Fusion::claim_rewards(
				RawOrigin::Signed(controller_address).into(),
				era_to_reward,
				BTC_POOL_ID,
				fusion_address2,
			));

			let era_rewards = EraRewards::<Test>::get(era_to_reward, BTC_POOL_ID).unwrap();
			assert_eq!(era_rewards.rewards, era_rewards.claimed_rewards);
			assert_eq!(
				era_rewards.additional_rewards,
				era_rewards.additional_claimed_rewards
			);
		});
	}

	#[test]
	fn exposure_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);

			assert_noop!(
				Fusion::claim_rewards(
					RawOrigin::Signed(controller_address).into(),
					1,
					BTC_POOL_ID,
					fusion_address,
				),
				Error::<Test>::ExposureNotFound
			);
		});
	}

	#[test]
	fn no_rewards_for_era() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);

			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			run_to_era(3);

			assert_noop!(
				Fusion::claim_rewards(
					RawOrigin::Signed(controller_address).into(),
					3,
					BTC_POOL_ID,
					fusion_address,
				),
				Error::<Test>::NoRewardsForEra
			);
		});
	}

	#[test]
	fn already_claimed() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);

			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			run_to_era(4);

			assert_ok!(Fusion::claim_rewards(
				RawOrigin::Signed(controller_address).into(),
				3,
				BTC_POOL_ID,
				fusion_address,
			));

			assert_noop!(
				Fusion::claim_rewards(
					RawOrigin::Signed(controller_address).into(),
					3,
					BTC_POOL_ID,
					fusion_address,
				),
				Error::<Test>::AlreadyClaimed
			);
		});
	}

	#[test]
	fn user_not_found_in_exposure() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address_1 = FusionAddress::EvmAddress(H160::repeat_byte(0x01));
			let controller_address_1 = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address_1, controller_address_1);
			Fusion::add_to_currency_balance(fusion_address_1, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address_1).into(),
				fusion_address_1,
				BTC_POOL_ID,
				100_000_000
			));

			let fusion_address_2 = FusionAddress::EvmAddress(H160::repeat_byte(0x02));
			let controller_address_2 = RANDOM_POT;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address_2, controller_address_2);

			run_to_era(4);

			assert_noop!(
				Fusion::claim_rewards(
					RawOrigin::Signed(controller_address_2).into(),
					3,
					BTC_POOL_ID,
					fusion_address_2
				),
				Error::<Test>::UserNotFoundInExposure
			);
		});
	}

	#[test]
	fn no_rewards_to_claim() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::repeat_byte(0x01));
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			let era_to_run = 4;
			run_to_era(era_to_run);
			let era_to_reward = era_to_run - 1;

			EraRewards::<Test>::insert(
				era_to_reward,
				BTC_POOL_ID,
				EraReward {
					rewards: 0,
					claimed_rewards: 0,
					additional_rewards: 0,
					additional_claimed_rewards: 0,
				},
			);

			assert_noop!(
				Fusion::claim_rewards(
					RawOrigin::Signed(controller_address).into(),
					era_to_reward,
					BTC_POOL_ID,
					fusion_address
				),
				Error::<Test>::NoRewardsToClaim
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::repeat_byte(0x01));
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			run_to_era(4);

			Currencies::<Test>::remove(AVAIL_CURRENCY_ID); // Used to convert avail_to_currency

			assert_noop!(
				Fusion::claim_rewards(
					RawOrigin::Signed(controller_address).into(),
					3,
					BTC_POOL_ID,
					fusion_address
				),
				Error::<Test>::CurrencyNotFound
			);
		});
	}

	#[test]
	fn not_enough_claimable_balance_in_pool() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::repeat_byte(0x01));
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			run_to_era(4);

			let pool_claimable_account = Fusion::get_pool_funds_account(BTC_POOL_ID);
			assert_ok!(Balances::force_set_balance(
				RawOrigin::Root.into(),
				pool_claimable_account,
				avail_core::currency::Balance::zero(),
			));

			assert_noop!(
				Fusion::claim_rewards(
					RawOrigin::Signed(controller_address).into(),
					3,
					BTC_POOL_ID,
					fusion_address
				),
				Error::<Test>::NotEnoughClaimableBalanceInPool
			);
		});
	}

	#[test]
	fn pool_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::repeat_byte(0x01));
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			run_to_era(4);

			Pools::<Test>::remove(AVAIL_POOL_ID);

			assert_noop!(
				Fusion::claim_rewards(
					RawOrigin::Signed(controller_address).into(),
					3,
					BTC_POOL_ID,
					fusion_address
				),
				Error::<Test>::PoolNotFound
			);
		});
	}
}

mod unbond_currency {
	use super::*;

	#[test]
	fn unbond_currency() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			let pool = create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			let staked_amount = 100_000_000;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			run_to_era(4);

			let partial_unbond_amount = 50_000_000;
			assert_ok!(Fusion::unbond_currency(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				partial_unbond_amount
			));

			let membership = Memberships::<Test>::get(fusion_address, BTC_POOL_ID).unwrap();
			assert_eq!(
				membership.active_points,
				pool.currency_to_points(partial_unbond_amount, None)
					.unwrap()
			);

			System::assert_has_event(RuntimeEvent::Fusion(Event::CurrencyUnbonded {
				fusion_address,
				pool_id: BTC_POOL_ID,
				currency_id: BTC_CURRENCY_ID,
				unbonded_amount: partial_unbond_amount,
				points: pool
					.currency_to_points(partial_unbond_amount, None)
					.unwrap(),
				era: 4,
			}));

			let full_unbond_amount = 50_000_000;
			assert_ok!(Fusion::unbond_currency(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				full_unbond_amount
			));

			let membership = Memberships::<Test>::get(fusion_address, BTC_POOL_ID).unwrap();
			assert_eq!(membership.active_points, 0);

			System::assert_last_event(RuntimeEvent::Fusion(Event::CurrencyUnbonded {
				fusion_address,
				pool_id: BTC_POOL_ID,
				currency_id: BTC_CURRENCY_ID,
				unbonded_amount: full_unbond_amount,
				points: pool.currency_to_points(full_unbond_amount, None).unwrap(),
				era: 4,
			}));
		});
	}

	#[test]
	fn invalid_substrate_address() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let staked_amount = 100_000_000;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, FUSION_STAKER);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(FUSION_STAKER).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			let unbond_amount = 50_000_000;
			let incorrect_controller_address = ALICE;
			assert_noop!(
				Fusion::unbond_currency(
					RawOrigin::Signed(incorrect_controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					unbond_amount
				),
				Error::<Test>::InvalidSubstrateAddress
			);
		});
	}

	#[test]
	fn membership_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let staked_amount = 100_000_000;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, FUSION_STAKER);

			assert_noop!(
				Fusion::unbond_currency(
					RawOrigin::Signed(FUSION_STAKER).into(),
					fusion_address,
					BTC_POOL_ID,
					staked_amount
				),
				Error::<Test>::MembershipNotFound
			);
		});
	}

	#[test]
	fn no_active_points_to_unbond() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, FUSION_STAKER);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(FUSION_STAKER).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			assert_ok!(Fusion::unbond_currency(
				RawOrigin::Signed(FUSION_STAKER).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			assert_noop!(
				Fusion::unbond_currency(
					RawOrigin::Signed(FUSION_STAKER).into(),
					fusion_address,
					BTC_POOL_ID,
					50_000_000
				),
				Error::<Test>::NoActivePointsToUnbond
			);
		});
	}

	#[test]
	fn pool_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, FUSION_STAKER);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(FUSION_STAKER).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			Pools::<Test>::remove(BTC_POOL_ID);

			assert_noop!(
				Fusion::unbond_currency(
					RawOrigin::Signed(FUSION_STAKER).into(),
					fusion_address,
					BTC_POOL_ID,
					50_000_000
				),
				Error::<Test>::PoolNotFound
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, FUSION_STAKER);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(FUSION_STAKER).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			Currencies::<Test>::remove(BTC_CURRENCY_ID);

			assert_noop!(
				Fusion::unbond_currency(
					RawOrigin::Signed(FUSION_STAKER).into(),
					fusion_address,
					BTC_POOL_ID,
					50_000_000
				),
				Error::<Test>::CurrencyNotFound
			);
		});
	}

	#[test]
	fn pool_is_not_destroying() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, FUSION_STAKER);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(FUSION_STAKER).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			assert_noop!(
				Fusion::do_unbond(fusion_address, BTC_POOL_ID, Some(50_000_000), true),
				Error::<Test>::PoolIsNotDestroying
			);
		});
	}

	#[test]
	fn invalid_amount() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, FUSION_STAKER);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(FUSION_STAKER).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			assert_noop!(
				Fusion::unbond_currency(
					RawOrigin::Signed(FUSION_STAKER).into(),
					fusion_address,
					BTC_POOL_ID,
					0
				),
				Error::<Test>::InvalidAmount
			);
		});
	}

	#[test]
	fn invalid_unbond_amount() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, FUSION_STAKER);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(FUSION_STAKER).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			assert_noop!(
				Fusion::unbond_currency(
					RawOrigin::Signed(FUSION_STAKER).into(),
					fusion_address,
					BTC_POOL_ID,
					200_000_000
				),
				Error::<Test>::InvalidUnbondAmount
			);
		});
	}

	#[test]
	fn amount_will_go_below_minimum() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, FUSION_STAKER);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(FUSION_STAKER).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			Currencies::<Test>::mutate(BTC_CURRENCY_ID, |currency| {
				if let Some(c) = currency {
					c.min_amount = 50_000_000;
				}
			});

			assert_noop!(
				Fusion::unbond_currency(
					RawOrigin::Signed(FUSION_STAKER).into(),
					fusion_address,
					BTC_POOL_ID,
					75_000_000
				),
				Error::<Test>::AmountWillGoBelowMinimum
			);
		});
	}

	#[test]
	fn pool_member_limit_reached() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			run_to_era(4);

			for i in 1..(1 + MaxMembersPerPool::get()) {
				let extra_address = FusionAddress::EvmAddress(H160::repeat_byte(i as u8));
				UnbondingChunks::<Test>::mutate(BTC_POOL_ID, 4, |chunks| {
					chunks.try_push((extra_address, 1)).unwrap();
				});
			}

			assert_noop!(
				Fusion::unbond_currency(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					50_000_000
				),
				Error::<Test>::PoolMemberLimitReached
			);
		});
	}

	#[test]
	fn max_unbonding_chunks_exceeded() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			run_to_era(4);

			Memberships::<Test>::mutate(fusion_address, BTC_POOL_ID, |membership| {
				if let Some(membership) = membership {
					for era in 100..(100 + MaxUnbonding::get()) {
						membership.unbonding_eras.try_push(era).unwrap();
					}
				}
			});

			assert_noop!(
				Fusion::unbond_currency(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID,
					50_000_000
				),
				Error::<Test>::MaxUnbondingChunksExceeded
			);
		});
	}
}

mod withdraw_unbonded_currency {
	use super::*;

	#[test]
	fn withdraw_unbonded_currency() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			assert_ok!(Fusion::unbond_currency(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			let bonding_duration = FusionBondingDuration::get();
			let active_era = Staking::active_era().unwrap().index;
			run_to_era(active_era + bonding_duration + 1);

			assert_ok!(Fusion::withdraw_unbonded_currency(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID
			));

			System::assert_has_event(RuntimeEvent::Fusion(Event::CurrencyWithdrawn {
				fusion_address,
				pool_id: BTC_POOL_ID,
				currency_id: BTC_CURRENCY_ID,
				amount: staked_amount,
			}));
			System::assert_has_event(RuntimeEvent::Fusion(Event::PoolMembershipRemoved {
				fusion_address,
				pool_id: BTC_POOL_ID,
			}));
		});
	}

	#[test]
	fn membership_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);

			assert_noop!(
				Fusion::withdraw_unbonded_currency(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID
				),
				Error::<Test>::MembershipNotFound
			);
		});
	}

	#[test]
	fn pool_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			Pools::<Test>::remove(BTC_POOL_ID);

			assert_noop!(
				Fusion::withdraw_unbonded_currency(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID
				),
				Error::<Test>::PoolNotFound
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			Currencies::<Test>::remove(BTC_CURRENCY_ID);

			assert_noop!(
				Fusion::withdraw_unbonded_currency(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID
				),
				Error::<Test>::CurrencyNotFound
			);
		});
	}

	#[test]
	fn pool_is_not_destroying() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			assert_noop!(
				Fusion::do_withdraw_unbonded_currency(fusion_address, BTC_POOL_ID, true),
				Error::<Test>::PoolIsNotDestroying
			);
		});
	}

	#[test]
	fn no_funds_to_withdraw() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			let bonding_duration = FusionBondingDuration::get();
			let active_era = Staking::active_era().unwrap().index;
			run_to_era(active_era + bonding_duration + 1);

			assert_noop!(
				Fusion::withdraw_unbonded_currency(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID
				),
				Error::<Test>::NoFundsToWithdraw
			);
		});
	}

	#[test]
	fn invalid_substrate_address() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let incorrect_controller_address = NOMINATOR_1;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, FUSION_STAKER);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_noop!(
				Fusion::withdraw_unbonded_currency(
					RawOrigin::Signed(incorrect_controller_address).into(),
					fusion_address,
					BTC_POOL_ID
				),
				Error::<Test>::InvalidSubstrateAddress
			);
		});
	}

	#[test]
	fn no_funds_to_withdraw_bonding_duration_not_passed() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			assert_ok!(Fusion::unbond_currency(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			run_to_era(3);

			assert_noop!(
				Fusion::withdraw_unbonded_currency(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BTC_POOL_ID
				),
				Error::<Test>::NoFundsToWithdraw
			);
		});
	}
}

mod unbond_currency_other {
	use super::*;

	#[test]
	fn unbond_currency_other() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let staked_amount = 100_000_000;
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			let mut pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			pool.state = FusionPoolState::Destroying;
			Pools::<Test>::insert(BTC_POOL_ID, pool);

			let other_address = ALICE;
			assert_ok!(Fusion::unbond_currency_other(
				RawOrigin::Signed(other_address).into(),
				fusion_address,
				BTC_POOL_ID
			));

			let unbonding_chunks = UnbondingChunks::<Test>::get(BTC_POOL_ID, 1);
			assert_eq!(unbonding_chunks.len(), 1);
			assert_eq!(unbonding_chunks[0].0, fusion_address);
			assert_eq!(unbonding_chunks[0].1, staked_amount);
		});
	}
}

mod withdraw_unbonded_currency_other {
	use super::*;

	#[test]
	fn withdraw_unbonded_currency_other() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let staked_amount = 100_000_000;
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			assert_ok!(Fusion::unbond_currency(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			let bonding_duration = FusionBondingDuration::get();
			let active_era = Staking::active_era().unwrap().index;
			run_to_era(active_era + bonding_duration);

			let mut pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			pool.state = FusionPoolState::Destroying;
			Pools::<Test>::insert(BTC_POOL_ID, pool);

			let other_address = ALICE;
			assert_ok!(Fusion::withdraw_unbonded_currency_other(
				RawOrigin::Signed(other_address).into(),
				fusion_address,
				BTC_POOL_ID
			));

			let balance = Fusion::user_currency_balances(fusion_address, BTC_CURRENCY_ID).unwrap();
			assert_eq!(balance.amount, staked_amount);
		});
	}
}

mod withdraw_avail_to_controller {
	use super::*;

	#[test]
	fn withdraw_avail_to_controller() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let staked_amount = 100_000_000;
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			assert_ok!(Fusion::set_compounding(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				false
			));

			run_to_era(4);

			assert_ok!(Fusion::claim_rewards(
				RawOrigin::Signed(controller_address).into(),
				3,
				BTC_POOL_ID,
				fusion_address
			));

			let controller_balance_before = Balances::free_balance(controller_address);

			assert_ok!(Fusion::withdraw_avail_to_controller(
				RawOrigin::Signed(controller_address).into(),
				fusion_address
			));

			let controller_balance_after = Balances::free_balance(controller_address);

			assert!(controller_balance_after > controller_balance_before);

			System::assert_last_event(RuntimeEvent::Fusion(Event::AvailWithdrawnToController {
				fusion_address,
				controller: controller_address,
				amount: controller_balance_after - controller_balance_before,
			}));
		});
	}

	#[test]
	fn invalid_substrate_address() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			assert_noop!(
				Fusion::withdraw_avail_to_controller(
					RawOrigin::Signed(controller_address).into(),
					fusion_address
				),
				Error::<Test>::InvalidSubstrateAddress
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let staked_amount = 100_000_000;
			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(
				fusion_address,
				AVAIL_CURRENCY_ID,
				staked_amount,
				false,
			)
			.unwrap();

			Currencies::<Test>::remove(AVAIL_CURRENCY_ID);

			assert_noop!(
				Fusion::withdraw_avail_to_controller(
					RawOrigin::Signed(controller_address).into(),
					fusion_address
				),
				Error::<Test>::CurrencyNotFound
			);
		});
	}

	#[test]
	fn no_controller_address_for_user() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let staked_amount = 100_000_000;
			let fusion_address = FusionAddress::EvmAddress(H160::zero());

			Fusion::add_to_currency_balance(
				fusion_address,
				AVAIL_CURRENCY_ID,
				staked_amount,
				false,
			)
			.unwrap();

			assert_noop!(
				Fusion::do_withdraw_avail_to_controller(fusion_address),
				Error::<Test>::NoControllerAddressForUser
			);
		});
	}

	#[test]
	fn no_currency_balance_for_user() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);

			assert_noop!(
				Fusion::withdraw_avail_to_controller(
					RawOrigin::Signed(controller_address).into(),
					fusion_address
				),
				Error::<Test>::NoCurrencyBalanceForUser
			);
		});
	}

	#[test]
	fn no_funds_to_withdraw() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, AVAIL_CURRENCY_ID, 0, false).unwrap();

			assert_noop!(
				Fusion::withdraw_avail_to_controller(
					RawOrigin::Signed(controller_address).into(),
					fusion_address
				),
				Error::<Test>::NoFundsToWithdraw
			);
		});
	}
}

mod set_pool_boost_allocations {
	use super::*;

	#[test]
	fn set_pool_boost_allocations() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None,
				None,
				ConfigOp::Noop,
				ConfigOp::Set((Perbill::from_percent(5), 1 * AVAIL)),
				None
			));
			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				AVAIL_POOL_ID,
				None,
				None,
				ConfigOp::Noop,
				ConfigOp::Set((Perbill::from_percent(5), 2 * AVAIL)),
				None
			));

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();
			Fusion::add_to_currency_balance(fusion_address, AVAIL_CURRENCY_ID, 3 * AVAIL, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				AVAIL_POOL_ID,
				3 * AVAIL
			));

			let random_address = ALICE;
			assert_ok!(Fusion::set_pool_boost_allocations(
				RawOrigin::Signed(random_address).into(),
				fusion_address,
				BoundedVec::try_from(vec![BTC_POOL_ID]).unwrap()
			));
			System::assert_last_event(RuntimeEvent::Fusion(Event::UserBoostAllocationsOptimized {
				fusion_address,
				pools_added: vec![BTC_POOL_ID],
				pools_removed: vec![],
			}));

			assert_ok!(Fusion::set_pool_boost_allocations(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BoundedVec::try_from(vec![BTC_POOL_ID, AVAIL_POOL_ID]).unwrap()
			));
			System::assert_last_event(RuntimeEvent::Fusion(Event::UserBoostAllocationsOptimized {
				fusion_address,
				pools_added: vec![AVAIL_POOL_ID],
				pools_removed: vec![],
			}));
		});
	}

	#[test]
	fn not_authorized() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				AVAIL_POOL_ID,
				None,
				None,
				ConfigOp::Noop,
				ConfigOp::Set((Perbill::from_percent(5), 1 * AVAIL)),
				None
			));

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, AVAIL_CURRENCY_ID, 1 * AVAIL, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				AVAIL_POOL_ID,
				1 * AVAIL
			));

			assert_ok!(Fusion::set_pool_boost_allocations(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BoundedVec::try_from(vec![AVAIL_POOL_ID]).unwrap()
			));

			let random_address = ALICE;
			assert_noop!(
				Fusion::set_pool_boost_allocations(
					RawOrigin::Signed(random_address).into(),
					fusion_address,
					BoundedVec::try_from(vec![BTC_POOL_ID]).unwrap()
				),
				Error::<Test>::NotAuthorized
			);
		});
	}

	#[test]
	fn no_avail_membership() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, AVAIL_CURRENCY_ID, 1 * AVAIL, false)
				.unwrap();

			assert_noop!(
				Fusion::set_pool_boost_allocations(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BoundedVec::try_from(vec![BTC_POOL_ID]).unwrap()
				),
				Error::<Test>::NoAvailMembership
			);
		});
	}

	#[test]
	fn currency_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None,
				None,
				ConfigOp::Noop,
				ConfigOp::Set((Perbill::from_percent(5), 1 * AVAIL)),
				None
			));

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, AVAIL_CURRENCY_ID, 1 * AVAIL, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				AVAIL_POOL_ID,
				1 * AVAIL
			));

			Currencies::<Test>::remove(AVAIL_CURRENCY_ID);

			assert_noop!(
				Fusion::set_pool_boost_allocations(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BoundedVec::try_from(vec![BTC_POOL_ID]).unwrap()
				),
				Error::<Test>::CurrencyNotFound
			);
		});
	}

	#[test]
	fn pool_not_found() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, AVAIL_CURRENCY_ID, 1 * AVAIL, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				AVAIL_POOL_ID,
				1 * AVAIL
			));

			Pools::<Test>::remove(AVAIL_POOL_ID);

			assert_noop!(
				Fusion::set_pool_boost_allocations(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BoundedVec::try_from(vec![BTC_POOL_ID]).unwrap()
				),
				Error::<Test>::PoolNotFound
			);
		});
	}

	#[test]
	fn pool_has_no_boost() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_POOL_ID, 10_000_000, false)
				.unwrap();
			Fusion::add_to_currency_balance(fusion_address, AVAIL_POOL_ID, 1 * AVAIL, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				10_000_000
			));
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				AVAIL_POOL_ID,
				1 * AVAIL
			));

			assert_noop!(
				Fusion::set_pool_boost_allocations(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BoundedVec::try_from(vec![BTC_POOL_ID]).unwrap()
				),
				Error::<Test>::PoolHasNoBoost
			);
		});
	}

	#[test]
	fn not_enough_avail_for_boost() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				None,
				None,
				ConfigOp::Noop,
				ConfigOp::Set((Perbill::from_percent(5), 2 * AVAIL)),
				None
			));
			assert_ok!(Fusion::set_pool(
				RawOrigin::Root.into(),
				AVAIL_POOL_ID,
				None,
				None,
				ConfigOp::Noop,
				ConfigOp::Set((Perbill::from_percent(5), 1 * AVAIL)),
				None
			));

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;

			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 10_000_000, false)
				.unwrap();
			Fusion::add_to_currency_balance(fusion_address, AVAIL_CURRENCY_ID, 1 * AVAIL, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_CURRENCY_ID,
				10_000_000
			));
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				AVAIL_POOL_ID,
				1 * AVAIL
			));

			assert_noop!(
				Fusion::set_pool_boost_allocations(
					RawOrigin::Signed(controller_address).into(),
					fusion_address,
					BoundedVec::try_from(vec![BTC_POOL_ID, AVAIL_POOL_ID]).unwrap()
				),
				Error::<Test>::NotEnoughAvailForBoost
			);
		});
	}
}

mod withdraw_pool_account {
	use super::*;

	#[test]
	fn withdraw_pool_account() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let dest_account = ALICE;
			let withdraw_amount = 1 * AVAIL;
			let initial_amount = Balances::free_balance(&dest_account);

			let mut pool = Pools::<Test>::get(AVAIL_POOL_ID).unwrap();
			pool.funds_account = Fusion::get_pool_funds_account(AVAIL_POOL_ID);
			Balances::make_free_balance_be(&pool.funds_account, 2 * AVAIL);

			Pools::<Test>::insert(AVAIL_POOL_ID, pool);

			assert_ok!(Fusion::withdraw_pool_account(
				RawOrigin::Root.into(),
				AVAIL_POOL_ID,
				withdraw_amount,
				dest_account
			));

			assert_eq!(
				Balances::free_balance(&dest_account),
				initial_amount + withdraw_amount
			);

			System::assert_last_event(RuntimeEvent::Fusion(Event::FundsAccountWithdrawn {
				recipient: dest_account,
				amount: withdraw_amount,
			}));
		});
	}

	#[test]
	fn bad_origin() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let dest_account = ALICE;

			assert_noop!(
				Fusion::withdraw_pool_account(
					RawOrigin::Signed(ALICE).into(),
					AVAIL_POOL_ID,
					1 * AVAIL,
					dest_account
				),
				BadOrigin
			);
		});
	}

	#[test]
	fn invalid_amount() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let dest_account = ALICE;

			assert_noop!(
				Fusion::withdraw_pool_account(
					RawOrigin::Root.into(),
					AVAIL_POOL_ID,
					0,
					dest_account
				),
				Error::<Test>::InvalidAmount
			);
		});
	}

	#[test]
	fn insufficient_balance() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();

			let dest_account = ALICE;
			let withdraw_amount = 1 * AVAIL;

			let mut pool = Pools::<Test>::get(AVAIL_POOL_ID).unwrap();
			pool.funds_account = Fusion::get_pool_funds_account(AVAIL_POOL_ID);
			Balances::make_free_balance_be(&pool.funds_account, withdraw_amount / 2);

			Pools::<Test>::insert(AVAIL_POOL_ID, pool);

			assert_noop!(
				Fusion::withdraw_pool_account(
					RawOrigin::Root.into(),
					AVAIL_POOL_ID,
					withdraw_amount,
					dest_account
				),
				TokenError::FundsUnavailable
			);
		});
	}
}

mod sanity_checks {
	use super::*;

	#[test]
	fn era_progresses_and_native_rewards_are_generated() {
		new_test_ext().execute_with(|| {
			// Get balance before payout
			let init_balance = Balances::free_balance(VALIDATOR_1);

			// Progress to the start of era 5
			run_to_era(5);

			// Check the active era
			assert_eq!(Staking::active_era().unwrap().index, 5);
			assert_eq!(Session::validators(), vec![VALIDATOR_1, VALIDATOR_2]);

			// Log staking-related storages
			for era in 1..=4 {
				assert!(Staking::eras_total_stake(era) > avail_core::currency::Balance::zero());
				let reward_points = Staking::eras_reward_points(era);
				let validators_with_reward = pallet_staking::ErasRewardPoints::<Test>::get(era)
					.individual
					.keys()
					.cloned()
					.collect::<Vec<_>>();
				assert!(reward_points.total > 0);
				assert!(validators_with_reward.len() > 0);
				assert!(
					Staking::eras_stakers(era, &VALIDATOR_1).total
						> avail_core::currency::Balance::zero()
				);
				assert!(
					Staking::eras_stakers(era, &VALIDATOR_2).total
						> avail_core::currency::Balance::zero()
				);
			}

			// Check that rewards are correctly generated
			let new_balance = Balances::free_balance(VALIDATOR_1);
			assert!(new_balance > init_balance);
		});
	}

	#[test]
	fn pool_receives_staking_rewards() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			let pool = create_btc_pool();

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));
			assert_ok!(Fusion::set_compounding(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				false,
			));

			// Era 1 - We start fusion staking
			// Era 2 - Fusion snapshot is done for Era 3 native snapshot
			// Era 3 - Native snapshot contains fusion stake
			// Era 4 - Fusion user can claim their rewards and pool receives actual staking rewards
			// We check that the pool account has received funds
			let init_balance = Balances::free_balance(pool.funds_account);
			run_to_era(4);
			let new_balance = Balances::free_balance(pool.funds_account);
			assert!(new_balance > init_balance);

			// Now we check that the fusion user can claim (the checks on numbers are done elsewhere)
			let init_fusion_balance =
				UserCurrencyBalances::<Test>::get(fusion_address, AVAIL_CURRENCY_ID);
			assert_ok!(Fusion::claim_rewards(
				RawOrigin::Signed(controller_address).into(),
				3,
				BTC_POOL_ID,
				fusion_address,
			));
			let new_fusion_balance =
				UserCurrencyBalances::<Test>::get(fusion_address, AVAIL_CURRENCY_ID);

			assert!(init_fusion_balance.is_none());
			assert!(new_fusion_balance.is_some());
		});
	}

	#[test]
	fn validator_gets_elected_with_fusion_stake() {
		new_test_ext().execute_with(|| {
			assert_eq!(Session::validators(), vec![VALIDATOR_1, VALIDATOR_2]);

			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();

			// Elect only validator 3
			assert_ok!(Fusion::nominate(
				RawOrigin::Root.into(),
				BTC_POOL_ID,
				vec![VALIDATOR_3].try_into().unwrap()
			));

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, 100_000_000, false)
				.unwrap();
			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				100_000_000
			));

			// Validator 3 should be elected during era 3
			run_to_era(3);
			assert_eq!(Session::validators(), vec![VALIDATOR_1, VALIDATOR_3]);

			run_to_era(10);
			assert_eq!(Session::validators(), vec![VALIDATOR_1, VALIDATOR_3]);
		});
	}
}

mod currency_conversion {
	use super::*;

	#[test]
	fn currency_to_avail() {
		new_test_ext().execute_with(|| {
			let (currency, _, _) = create_avail_currency();
			let era = Staking::active_era().unwrap().index;
			let avail_in_fusion_currency: FusionCurrencyBalance = 1_000_000_000_000_000_000;

			assert!(matches!(
				currency.currency_to_avail(AVAIL_CURRENCY_ID, avail_in_fusion_currency, None),
				Ok(AVAIL)
			));

			CurrencyRates::<Test>::remove(era, AVAIL_CURRENCY_ID);
			assert!(matches!(
				currency.currency_to_avail(AVAIL_CURRENCY_ID, avail_in_fusion_currency, None),
				Err(Error::<Test>::CurrencyRateNotFound)
			));
		})
	}

	#[test]
	fn avail_to_currency() {
		new_test_ext().execute_with(|| {
			let (currency, _, _) = create_avail_currency();
			let era = Staking::active_era().unwrap().index;

			assert!(matches!(
				currency.avail_to_currency(AVAIL_CURRENCY_ID, AVAIL, None),
				Ok(1_000_000_000_000_000_000)
			));

			CurrencyRates::<Test>::remove(era, AVAIL_CURRENCY_ID);
			assert!(matches!(
				currency.currency_to_avail(AVAIL_CURRENCY_ID, AVAIL, None),
				Err(Error::<Test>::CurrencyRateNotFound)
			));
		})
	}

	#[test]
	fn points_to_currency() {
		new_test_ext().execute_with(|| {
			let (currency, _, _) = create_avail_currency();
			let mut pool = create_avail_pool();

			let one_point: Points = 1_000_000_000_000_000_000;
			let amount: FusionCurrencyBalance = 1_000_000_000_000_000_000;

			assert_eq!(
				pool.points_to_currency(one_point, Some(&currency)).unwrap(),
				amount
			);

			Currencies::<Test>::remove(AVAIL_CURRENCY_ID);

			assert!(matches!(
				pool.points_to_currency(one_point, None),
				Err(Error::<Test>::CurrencyNotFound)
			));

			Currencies::<Test>::insert(AVAIL_CURRENCY_ID, currency);

			pool.total_staked_native = amount;
			pool.total_staked_points = one_point;
			Pools::<Test>::insert(AVAIL_POOL_ID, pool);

			let pool = Pools::<Test>::get(AVAIL_POOL_ID).unwrap();
			let currency = Currencies::<Test>::get(AVAIL_CURRENCY_ID).unwrap();

			assert_eq!(
				pool.points_to_currency(one_point, Some(&currency)).unwrap(),
				amount
			)
		})
	}

	#[test]
	fn currency_to_points() {
		new_test_ext().execute_with(|| {
			let (currency, _, _) = create_avail_currency();
			let mut pool = create_avail_pool();

			let one_point: Points = 1_000_000_000_000_000_000;
			let amount: FusionCurrencyBalance = 1_000_000_000_000_000_000;

			assert_eq!(
				pool.currency_to_points(amount, Some(&currency)).unwrap(),
				one_point
			);

			Currencies::<Test>::remove(AVAIL_CURRENCY_ID);

			assert!(matches!(
				pool.currency_to_points(amount, None),
				Err(Error::<Test>::CurrencyNotFound)
			));

			Currencies::<Test>::insert(AVAIL_CURRENCY_ID, currency);

			pool.total_staked_native = amount;
			pool.total_staked_points = one_point;
			Pools::<Test>::insert(AVAIL_POOL_ID, pool);

			let pool = Pools::<Test>::get(AVAIL_POOL_ID).unwrap();
			let currency = Currencies::<Test>::get(AVAIL_CURRENCY_ID).unwrap();

			assert_eq!(
				pool.currency_to_points(amount, Some(&currency)).unwrap(),
				one_point
			)
		})
	}

	#[test]
	fn points_to_avail() {
		new_test_ext().execute_with(|| {
			let (currency, _, _) = create_avail_currency();
			let mut pool = create_avail_pool();

			let one_point: Points = 1_000_000_000_000_000_000;
			let amount: FusionCurrencyBalance = 1_000_000_000_000_000_000;

			assert_eq!(
				pool.points_to_avail(one_point, AVAIL_CURRENCY_ID, None, None)
					.unwrap(),
				AVAIL
			);

			Currencies::<Test>::remove(AVAIL_CURRENCY_ID);

			assert!(matches!(
				pool.points_to_avail(one_point, AVAIL_CURRENCY_ID, None, None),
				Err(Error::<Test>::CurrencyNotFound)
			));

			Currencies::<Test>::insert(AVAIL_CURRENCY_ID, currency);

			pool.total_staked_native = amount;
			pool.total_staked_points = one_point;
			Pools::<Test>::insert(AVAIL_POOL_ID, pool);

			let pool = Pools::<Test>::get(AVAIL_POOL_ID).unwrap();

			assert_eq!(
				pool.points_to_avail(one_point, AVAIL_CURRENCY_ID, None, None)
					.unwrap(),
				amount
			)
		})
	}

	#[test]
	fn avail_to_points() {
		new_test_ext().execute_with(|| {
			let (currency, _, _) = create_avail_currency();
			let mut pool = create_avail_pool();

			let one_point: Points = 1_000_000_000_000_000_000;
			let amount: FusionCurrencyBalance = 1_000_000_000_000_000_000;

			assert_eq!(
				pool.avail_to_points(AVAIL, AVAIL_CURRENCY_ID, None, None)
					.unwrap(),
				AVAIL
			);

			Currencies::<Test>::remove(AVAIL_CURRENCY_ID);

			assert!(matches!(
				pool.avail_to_points(AVAIL, AVAIL_CURRENCY_ID, None, None),
				Err(Error::<Test>::CurrencyNotFound)
			));

			Currencies::<Test>::insert(AVAIL_CURRENCY_ID, currency);

			pool.total_staked_native = amount;
			pool.total_staked_points = one_point;
			Pools::<Test>::insert(AVAIL_POOL_ID, pool);

			let pool = Pools::<Test>::get(AVAIL_POOL_ID).unwrap();

			assert_eq!(
				pool.avail_to_points(AVAIL, AVAIL_CURRENCY_ID, None, None)
					.unwrap(),
				amount
			)
		})
	}
}

mod slashing {
	use super::*;

	#[test]
	fn fusion_slashing_works() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			let era_to_run = 4;
			run_to_end_of_era(era_to_run);

			// Add a slash
			add_slash(&VALIDATOR_1, Perbill::from_percent(10));
			System::assert_has_event(RuntimeEvent::Fusion(Event::FusionSlashReported {
				pool_id: BTC_POOL_ID,
				slash_era: era_to_run,
				slash_ratio: Perbill::from_parts(62_504_462),
				validator: 100,
			}));

			// Add a slash with bigger fraction, should add a new slash
			add_slash(&VALIDATOR_1, Perbill::from_percent(90));
			System::assert_has_event(RuntimeEvent::Fusion(Event::FusionSlashReported {
				pool_id: BTC_POOL_ID,
				slash_era: era_to_run,
				slash_ratio: Perbill::from_parts(500_035_701),
				validator: 100,
			}));

			let pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			assert!(pool.total_slashed_native == 0);
			assert!(pool.pending_slashes.len() == 2);
			assert!(
				HasPendingSlash::<Test>::get(era_to_run, (VALIDATOR_1, pool.funds_account)) == 2
			);

			let apply_era = SlashDeferDuration::get() + era_to_run + 1;

			assert_ok!(Staking::cancel_deferred_slash(
				RawOrigin::Root.into(),
				apply_era,
				vec![0]
			));

			System::assert_has_event(RuntimeEvent::Fusion(Event::FusionSlashCancelled {
				pool_ids: [BTC_POOL_ID].to_vec(),
				slash_era: era_to_run,
				validators: [100].to_vec(),
			}));

			// Check that the slash is indeed canceled
			let pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			assert!(pool.total_slashed_native == 0);
			assert_eq!(pool.pending_slashes.len(), 1);
			assert!(
				HasPendingSlash::<Test>::get(era_to_run, (VALIDATOR_1, pool.funds_account)) == 1
			);

			// Go to the apply era, remaining slash should get applied
			run_to_era(SlashDeferDuration::get() + era_to_run + 1);

			System::assert_has_event(RuntimeEvent::Fusion(Event::FusionPoolSlashed {
				currency_id: BTC_CURRENCY_ID,
				pool_id: BTC_POOL_ID,
				slash_era: era_to_run,
				amount: 50003570,
			}));

			let pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			assert!(pool.total_slashed_native == 50003570);
			assert!(pool.pending_slashes.len() == 0);
			assert!(
				HasPendingSlash::<Test>::get(era_to_run, (VALIDATOR_1, pool.funds_account)) == 0
			);
		})
	}

	#[test]
	fn fusion_slashing_works_on_unbonding_funds() {
		new_test_ext().execute_with(|| {
			create_avail_currency();
			create_avail_pool();
			create_btc_currency();
			create_btc_pool();
			let staked_amount = 100_000_000;

			let fusion_address = FusionAddress::EvmAddress(H160::zero());
			let controller_address = FUSION_STAKER;
			FusionAddressToSubstrateAddress::<Test>::insert(fusion_address, controller_address);
			Fusion::add_to_currency_balance(fusion_address, BTC_CURRENCY_ID, staked_amount, false)
				.unwrap();

			assert_ok!(Fusion::stake(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));

			let era_to_run = 4;
			run_to_era(4);
			assert_ok!(Fusion::unbond_currency(
				RawOrigin::Signed(controller_address).into(),
				fusion_address,
				BTC_POOL_ID,
				staked_amount
			));
			let pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			let old_unbonding_amount = pool.total_unbonding_native;

			run_to_end_of_era(era_to_run);
			add_slash(&VALIDATOR_1, Perbill::from_percent(10));
			run_to_era(SlashDeferDuration::get() + era_to_run + 1);

			let pool = Pools::<Test>::get(BTC_POOL_ID).unwrap();
			let new_unbonding_amount = pool.total_unbonding_native;
			assert!(old_unbonding_amount > new_unbonding_amount);
		})
	}
}
