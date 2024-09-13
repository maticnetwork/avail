use avail_core::{BlockLengthColumns, BlockLengthRows, NORMAL_DISPATCH_RATIO_PCT};
use frame_support::{
	construct_runtime, parameter_types,
	traits::Randomness,
	weights::{
		constants::WEIGHT_REF_TIME_PER_SECOND, ConstantMultiplier, IdentityFee, RuntimeDbWeight,
		Weight,
	},
};
use frame_system::{CheckEra, CheckNonce, CheckWeight};
use pallet_transaction_payment::CurrencyAdapter;
use sp_runtime::traits::{BlakeTwo256, ConstU32, IdentityLookup, TrailingZeroInput};
use sp_std::marker::PhantomData;

pub mod custom;
pub mod test_xt;

// Common Runtime Types
//

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u32;
pub type Moment = u64;
pub type Header = avail_core::header::Header;
pub type Signature = sp_runtime::testing::sr25519::Signature;
pub type TestXt = test_xt::TestXt<RuntimeCall, SignedExtra>;
pub type UncheckedExtrinsic = TestXt;
type Block = frame_system::mocking::MockDaBlock<Test>;
pub type SignedExtra = (
	CheckEra<Runtime>,
	CheckNonce<Runtime>,
	CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	da_control::CheckAppId<Runtime>,
);

/// Provides an implementation of [`frame_support::traits::Randomness`] that should only be used in
/// tests!
pub struct TestRandomness<T>(PhantomData<T>);

impl<Output: codec::Decode + Default, T> Randomness<Output, T::BlockNumber> for TestRandomness<T>
where
	T: frame_system::Config,
{
	fn random(subject: &[u8]) -> (Output, T::BlockNumber) {
		(
			Output::decode(&mut TrailingZeroInput::new(subject)).unwrap_or_default(),
			frame_system::Pallet::<T>::block_number(),
		)
	}
}

// Parameters

// Will contain `true` when the custom runtime logic was called.
pub const CUSTOM_ON_RUNTIME_KEY: &[u8] = b":custom:on_runtime";

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_babe` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 20000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10 * MINUTES;
pub const EPOCH_DURATION_IN_SLOTS: u64 = {
	const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

	(EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
};

/// We allow for 2 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_ref_time(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2)).set_proof_size(u64::MAX);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
			frame_system::limits::BlockWeights::with_sensible_defaults(MAXIMUM_BLOCK_WEIGHT, Perbill::from_percent(NORMAL_DISPATCH_RATIO_PCT as u32));
	pub const DbWeight: RuntimeDbWeight = RuntimeDbWeight {
		read: 10,
		write: 100,
	};
	pub const TransactionByteFee: Balance = 1;
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const ExistentialDeposit: Balance = 1;
	pub const MaxAuthorities: u32 = 100;
	pub const SessionsPerEra: sp_staking::SessionIndex = 6;
	pub const BondingDuration: sp_staking::EraIndex = 24 * 28;
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();

	// DA Control
	pub const MaxAppKeyLength :u32 = 64;
	pub const MaxAppDataLength :u32 = 512 * 1024; // 512 Kb
	pub const MinBlockRows: BlockLengthRows = BlockLengthRows(32);
	pub const MaxBlockRows: BlockLengthRows = BlockLengthRows(1024);
	pub const MinBlockCols: BlockLengthColumns = BlockLengthColumns(32);
	pub const MaxBlockCols: BlockLengthColumns = kate::config::MAX_BLOCK_COLUMNS;
}

#[derive(Clone, Copy)]
pub struct RuntimeVersion;
impl frame_support::traits::Get<sp_version::RuntimeVersion> for RuntimeVersion {
	fn get() -> sp_version::RuntimeVersion {
		RuntimeVersionTestValues::get()
	}
}

parameter_types! {
	pub static RuntimeVersionTestValues: sp_version::RuntimeVersion =
		Default::default();
}

// Runtime
//
impl frame_system::Config for Runtime {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type Block = Block;
	type BlockWeights = BlockWeights;
	type DbWeight = ();
	type Hash = sp_core::H256;
	type Hashing = BlakeTwo256;
	type HeaderExtensionBuilder =
		frame_system::native::hosted_header_builder::da::HeaderExtensionBuilder<Runtime>;
	type Nonce = u64;
	type Lookup = IdentityLookup<u64>;
	type MaxConsumers = ConstU32<16>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type PalletInfo = PalletInfo;
	type Randomness = TestRandomness<Runtime>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type SS58Prefix = ();
	type HeaderExtensionDataFilter = ();
	type SystemWeightInfo = ();
	type Version = RuntimeVersion;
}

#[derive_impl(pallet_transaction_payment::config_preludes::TestDefaultConfig as pallet_transaction_payment::DefaultConfig)]
impl pallet_transaction_payment::Config for Runtime {
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	type WeightToFee = IdentityFee<Balance>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig as pallet_balances::DefaultConfig)]
impl pallet_balances::Config for Runtime {
	type AccountStore = System;
	type Balance = Balance;
	type ExistentialDeposit = ExistentialDeposit;
}

#[derive_impl(da_control::config_preludes::TestDefaultConfig)]
impl da_control::Config for Runtime {}

impl custom::custom::Config for Runtime {}

construct_runtime!(
	pub enum Runtime
	{
		System: frame_system,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		Custom: custom::custom,
		DataAvailability: da_control,
	}
);
