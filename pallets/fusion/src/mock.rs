use crate::{
	self as pallet_fusion,
	types::{FusionAddress, PoolId},
	PoolAccountProvider,
};
use avail_core::currency::AVAIL;
use frame_election_provider_support::{
	bounds::{ElectionBounds, ElectionBoundsBuilder},
	onchain, SequentialPhragmen,
};
use frame_support::{
	assert_ok, derive_impl, parameter_types,
	traits::{FindAuthor, OnFinalize, OnInitialize, OneSessionHandler},
	PalletId,
};
use frame_system::{EnsureRoot, RawOrigin};
use pallet_staking::{ErasRewardPoints, ExposureOf, StashOf, UnappliedSlash};
use sp_core::{ConstU32, ConstU64, H160};
use sp_runtime::{curve::PiecewiseLinear, testing::UintAuthorityId, BuildStorage, Perbill};
use sp_staking::{
	offence::{DisableStrategy, OffenceDetails, OnOffenceHandler},
	EraIndex, Exposure, SessionIndex, StakerStatus,
};

type Extrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockDaBlock<Test>;
type Balance = u128;
type AccountId = u64;

pub const INIT_TIMESTAMP: u64 = 30_000;
pub const BLOCK_TIME: u64 = 20_000;

// Accounts used in tests
pub const ALICE: u64 = 1;
pub const POOL_NOMINATOR_ROLE_ACCOUNT: u64 = 10;
pub const VALIDATOR_1: u64 = 100;
pub const VALIDATOR_2: u64 = 200;
pub const VALIDATOR_3: u64 = 300;
pub const NOMINATOR_1: u64 = 400;
pub const NOMINATOR_2: u64 = 500;
pub const NOMINATOR_3: u64 = 600;
pub const REWARD_CLAIMER: u64 = 700;
pub const RANDOM_POT: u64 = 800;
pub const FUSION_STAKER: u64 = 900;
pub const SLASH_DESTINATION: u64 = 1000;

// IDs and data used in tests
pub const INVALID_ID: u32 = 9999;
pub const BTC_CURRENCY_ID: u32 = 1;
pub const BTC_POOL_ID: u32 = 1;
pub const BTC_DECIMAL: u8 = 8;

frame_support::construct_runtime!(
	pub struct Test {
		System: frame_system,
		Authorship: pallet_authorship,
		Timestamp: pallet_timestamp,
		Balances: pallet_balances,
		Staking: pallet_staking,
		Session: pallet_session,
		Historical: pallet_session::historical,
		Fusion: pallet_fusion,
	}
);

pub struct DeterministicAuthor;
impl FindAuthor<AccountId> for DeterministicAuthor {
	fn find_author<'a, I>(_digests: I) -> Option<AccountId>
	where
		I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
	{
		let validators = Session::validators();
		if validators.is_empty() {
			return None;
		}
		let current_block = System::block_number();
		let validator_index = (current_block as usize) % validators.len();
		Some(validators[validator_index])
	}
}

parameter_types! {
	pub const BlockHashCount: u32 = 250;
	pub static ExistentialDeposit: u128 = 1;
	pub static ElectionsBounds: ElectionBounds = ElectionBoundsBuilder::default().build();
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountData = pallet_balances::AccountData<Balance>;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type HeaderExtensionBuilder =
		frame_system::native::hosted_header_builder::da::HeaderExtensionBuilder<Test>;
	type OnSetCode = ();
	type PalletInfo = PalletInfo;
	type Randomness = frame_system::test_utils::TestRandomness<Test>;
	type Extrinsic = Extrinsic;
}

impl pallet_authorship::Config for Test {
	type FindAuthor = DeterministicAuthor;
	type EventHandler = ();
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig as pallet_balances::DefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = Balance;
	type ExistentialDeposit = ExistentialDeposit;
}

pub struct OtherSessionHandler;
impl OneSessionHandler<AccountId> for OtherSessionHandler {
	type Key = UintAuthorityId;

	fn on_genesis_session<'a, I: 'a>(_: I)
	where
		I: Iterator<Item = (&'a AccountId, Self::Key)>,
		AccountId: 'a,
	{
	}

	fn on_new_session<'a, I: 'a>(_: bool, _: I, _: I)
	where
		I: Iterator<Item = (&'a AccountId, Self::Key)>,
		AccountId: 'a,
	{
	}

	fn on_disabled(_validator_index: u32) {}
}

impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
	type Public = UintAuthorityId;
}
sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub other: OtherSessionHandler,
	}
}
impl pallet_session::Config for Test {
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type Keys = SessionKeys;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionHandler = (OtherSessionHandler,);
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = StashOf<Test>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type WeightInfo = ();
}
impl pallet_session::historical::Config for Test {
	type FullIdentification = Exposure<AccountId, Balance>;
	type FullIdentificationOf = ExposureOf<Test>;
}

pallet_staking_reward_curve::build! {
	const I_NPOS: sp_runtime::curve::PiecewiseLinear<'static> = curve!(
		min_inflation: 0_010_000, // minimum_inflation_rate = 1%
		max_inflation: 0_050_000, // maximum_inflation_rate = 5%
		ideal_stake: 0_500_000, // target_staking_rate = 50%
		falloff: 0_050_000,  // inflation_decay = 5%
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Test;
	type Solver = SequentialPhragmen<AccountId, Perbill>;
	type DataProvider = Staking;
	type WeightInfo = ();
	type MaxWinners = ConstU32<100>;
	type Bounds = ElectionsBounds;
}

parameter_types! {
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &I_NPOS;
	pub static Period: u32 = 2; // Number of blocks per session
	pub static Offset: u32 = 0; // Block where session starts
	pub static SessionsPerEra: SessionIndex = 3;
	pub static SlashDeferDuration: EraIndex = 6;
	pub const BondingDuration: EraIndex = 28;
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(75);
}

impl pallet_staking::Config for Test {
	type Currency = Balances;
	type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
	type UnixTime = pallet_timestamp::Pallet<Test>;
	type CurrencyToVote = ();
	type RewardRemainder = ();
	type RuntimeEvent = RuntimeEvent;
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type BondingDuration = BondingDuration;
	type SessionInterface = ();
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = ();
	type MaxExposurePageSize = ConstU32<64>;
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GenesisElectionProvider = Self::ElectionProvider;
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
	type MaxUnlockingChunks = ConstU32<32>;
	type HistoryDepth = ConstU32<84>;
	type MaxControllersInDeprecationBatch = ConstU32<100>;
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type EventListeners = ();
	type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
	type WeightInfo = ();
	type FusionExt = Fusion;
}

pub struct MockStakingFusionDataProvider;
impl pallet_fusion::StakingFusionDataProvider<Test> for MockStakingFusionDataProvider {
	fn active_era() -> EraIndex {
		pallet_staking::Pallet::<Test>::active_era()
			.map(|era_info| era_info.index)
			.unwrap_or(0)
	}

	fn current_era() -> EraIndex {
		pallet_staking::Pallet::<Test>::current_era().unwrap_or_default()
	}

	fn is_valid_validator(account: &AccountId) -> bool {
		pallet_staking::Validators::<Test>::contains_key(account)
			&& !pallet_staking::Validators::<Test>::get(account).blocked
	}

	fn has_earned_era_points(era: EraIndex, accounts: &Vec<AccountId>) -> bool {
		let era_points = pallet_staking::ErasRewardPoints::<Test>::get(era).individual;
		accounts
			.iter()
			.any(|account| era_points.contains_key(account))
	}

	fn unapplied_slashes(era: EraIndex) -> Vec<UnappliedSlash<AccountId, Balance>> {
		pallet_staking::UnappliedSlashes::<Test>::get(era)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add_dummy_validator(account: AccountId) {
		pallet_session::Validators::<Test>::mutate(|validators| {
			validators.push(account.clone());
		});

		let default_prefs = pallet_staking::ValidatorPrefs {
			commission: Perbill::from_percent(0),
			blocked: false,
		};
		pallet_staking::Validators::<Test>::insert(account.clone(), default_prefs);
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add_dummy_era_points(validator: AccountId, era: EraIndex) {
		pallet_staking::ErasRewardPoints::<Test>::mutate(era, |reward_points| {
			let points = reward_points
				.individual
				.entry(validator.clone())
				.or_insert(0);
			*points += 100;
		});
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn set_dummy_active_era(era: EraIndex) {
		pallet_staking::ActiveEra::<Test>::mutate(|active_era| {
			*active_era = Some(pallet_staking::ActiveEraInfo {
				index: era,
				start: None,
			});
		});
	}
}

pub struct MockPoolAccountProvider;
impl PoolAccountProvider<Test> for MockPoolAccountProvider {
	fn get_pool_funds_account(id: PoolId) -> AccountId {
		format!("99991{id}").parse().unwrap()
	}

	fn get_pool_claimable_account(id: PoolId) -> AccountId {
		format!("99992{id}").parse().unwrap()
	}
}

parameter_types! {
	pub const FusionPalletId: PalletId = PalletId(*b"avl/fusi");
	pub const MaxCurrencyNameLength: u32 = 32;
	pub const MaxMembersPerPool: u32 = 10;
	pub const MaxTargets: u32 = 16;
	pub const MaxUnbonding: u32 = 8;
	pub const FusionBondingDuration: EraIndex = 7;
	pub const HistoryDepth: u32 = 84;
	pub const MaxSlashesPerPool: u32 = 100;
	pub const MaxPoolsPerValidator: u32 = 100;
}
impl pallet_fusion::Config for Test {
	type Currency = Balances;
	type CurrencyToVote = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type ApprovedOrigin = EnsureRoot<AccountId>;
	type PalletId = FusionPalletId;
	type MaxCurrencyNameLength = MaxCurrencyNameLength;
	type MaxMembersPerPool = MaxMembersPerPool;
	type MaxTargets = MaxTargets;
	type MaxUnbonding = MaxUnbonding;
	type MaxPoolsPerValidator = MaxPoolsPerValidator;
	type MaxSlashesPerPool = MaxSlashesPerPool;
	type BondingDuration = FusionBondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type RewardRemainder = ();
	type HistoryDepth = HistoryDepth;
	type StakingFusionDataProvider = MockStakingFusionDataProvider;
	type PoolAccountProvider = MockPoolAccountProvider;
	type WeightInfo = ();
}

fn init_pallet() {
	assert_ok!(Fusion::update_max_tvl(
		RawOrigin::Root.into(),
		10_000_000 * AVAIL
	));
	let slash_destination = FusionAddress::EvmAddress(H160::repeat_byte(0x09));
	assert_ok!(Fusion::set_slash_destination(
		RawOrigin::Root.into(),
		Some(slash_destination),
		Some(SLASH_DESTINATION)
	));
	run_to_era(1);
}

/// Create new externalities for `Fusion` module tests.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(VALIDATOR_1, 10_000 * AVAIL),
			(VALIDATOR_2, 10_000 * AVAIL),
			(VALIDATOR_3, 10_000 * AVAIL),
			(NOMINATOR_1, 10_000 * AVAIL),
			(NOMINATOR_2, 10_000 * AVAIL),
			(NOMINATOR_3, 10_000 * AVAIL),
			(REWARD_CLAIMER, 10_000 * AVAIL),
			(RANDOM_POT, 10_000 * AVAIL),
			(FUSION_STAKER, 10_000 * AVAIL),
			(ALICE, 10_000 * AVAIL),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let _ = pallet_staking::GenesisConfig::<Test> {
		stakers: vec![
			(
				VALIDATOR_1,
				VALIDATOR_1,
				11 * AVAIL,
				StakerStatus::Validator,
			),
			(
				VALIDATOR_2,
				VALIDATOR_2,
				10 * AVAIL,
				StakerStatus::Validator,
			),
			(
				VALIDATOR_3,
				VALIDATOR_3,
				1 * AVAIL, // Not enough to be elected
				StakerStatus::Validator,
			),
			(
				NOMINATOR_1,
				NOMINATOR_1,
				1 * AVAIL,
				StakerStatus::Nominator(vec![VALIDATOR_1]),
			),
		],
		validator_count: 2,
		minimum_validator_count: 1,
		slash_reward_fraction: Perbill::from_percent(10),
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let _ = pallet_session::GenesisConfig::<Test> {
		keys: vec![
			(
				VALIDATOR_1,
				VALIDATOR_1,
				SessionKeys {
					other: UintAuthorityId::from(VALIDATOR_1),
				},
			),
			(
				VALIDATOR_2,
				VALIDATOR_2,
				SessionKeys {
					other: UintAuthorityId::from(VALIDATOR_2),
				},
			),
			(
				VALIDATOR_3,
				VALIDATOR_3,
				SessionKeys {
					other: UintAuthorityId::from(VALIDATOR_3),
				},
			),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
		Staking::on_initialize(1);
		Session::on_initialize(1);
		Fusion::on_initialize(1);
		Timestamp::set_timestamp(INIT_TIMESTAMP);
		init_pallet()
	});

	ext
}

pub fn run_to_block(n: u32) {
	let mut current_block = System::block_number();
	while current_block < n {
		let author = DeterministicAuthor::find_author(None).expect("A valid author should exist.");

		if let Some(active_era) = Staking::active_era() {
			let mut reward_points = Staking::eras_reward_points(active_era.index);
			*reward_points.individual.entry(author).or_insert(0) += 1;
			reward_points.total += 1;
			ErasRewardPoints::<Test>::insert(active_era.index, reward_points);
		}

		// Make validators claims their payouts if we're are at a new era block + 1
		let era_duration = Offset::get() + (Period::get() * SessionsPerEra::get());
		if current_block % era_duration == 0 {
			let era = Staking::active_era().unwrap().index;
			if era > 1 {
				make_all_reward_payment(era.saturating_sub(1));
			}
		}

		Balances::on_finalize(current_block);
		Authorship::on_finalize(current_block);
		Session::on_finalize(current_block);
		Staking::on_finalize(current_block);
		System::on_finalize(current_block);
		current_block += 1;
		System::set_block_number(current_block);
		System::on_initialize(current_block);
		Timestamp::set_timestamp(current_block as u64 * BLOCK_TIME + INIT_TIMESTAMP);
		Staking::on_initialize(current_block);
		Session::on_initialize(current_block);
		Authorship::on_initialize(current_block);
		Balances::on_initialize(current_block);
	}
}

// Run to the start of target_era + 1 block
pub fn run_to_era(target_era: u32) {
	let end = Offset::get() + (target_era * Period::get() * SessionsPerEra::get()) + 1;
	run_to_block(end);
	assert_eq!(Staking::active_era().unwrap().index, target_era);
}

// Run to the end of target_era (target_era + 1 era - 1 block)
pub fn run_to_end_of_era(target_era: u32) {
	let end = Offset::get() + ((target_era + 1) * Period::get() * SessionsPerEra::get()) - 1;
	run_to_block(end);
	assert_eq!(Staking::active_era().unwrap().index, target_era);
}

/// Make all validator and nominator request their payment
pub fn make_all_reward_payment(era: EraIndex) {
	let validators_with_reward = ErasRewardPoints::<Test>::get(era)
		.individual
		.keys()
		.cloned()
		.collect::<Vec<_>>();

	for validator in validators_with_reward.iter().filter_map(Staking::bonded) {
		assert_ok!(Staking::payout_stakers_by_page(
			RuntimeOrigin::signed(REWARD_CLAIMER),
			validator,
			era,
			0
		));
	}
}

pub fn on_offence_now(
	offenders: &[OffenceDetails<
		AccountId,
		pallet_session::historical::IdentificationTuple<Test>,
	>],
	slash_fraction: &[Perbill],
) {
	let now = Staking::active_era().unwrap().index;
	let _ = Staking::on_offence(
		offenders,
		slash_fraction,
		Staking::eras_start_session_index(now).unwrap(),
		DisableStrategy::WhenSlashed,
	);
}

pub fn add_slash(who: &AccountId, slash_fraction: Perbill) {
	on_offence_now(
		&[OffenceDetails {
			offender: (
				*who,
				Staking::eras_stakers(Staking::active_era().unwrap().index, who),
			),
			reporters: vec![],
		}],
		&[slash_fraction],
	);
}
