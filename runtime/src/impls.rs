use crate::{
	constants, prod_or_fast, voter_bags, weights, AccountId, AccountIndex, Babe, Balances, Block,
	BlockNumber, ElectionProviderMultiPhase, Everything, Fusion, Hash, Header, Historical,
	ImOnline, ImOnlineId, Index, Indices, Moment, NominationPools, Offences, OriginCaller,
	PalletInfo, Preimage, ReserveIdentifier, Runtime, RuntimeCall, RuntimeEvent,
	RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin, RuntimeVersion, Session, SessionKeys,
	Signature, SignedPayload, Staking, System, Timestamp, TransactionPayment, Treasury, TxPause,
	UncheckedExtrinsic, VoterList, MINUTES, SLOT_DURATION, VERSION,
};
use avail_core::{
	currency::{Balance, AVAIL, CENTS, NANO_AVAIL, PICO_AVAIL},
	AppId, NORMAL_DISPATCH_RATIO,
};
use pallet_staking::UnappliedSlash;

use crate::Vec;
use codec::{Decode, Encode, MaxEncodedLen};
use constants::time::DAYS;
use frame_election_provider_support::{
	onchain, BalancingConfig, ElectionDataProvider, SequentialPhragmen, VoteWeight,
};
use frame_support::{
	derive_impl,
	pallet_prelude::{Get, Weight},
	parameter_types,
	traits::{
		fungible::{Balanced, Credit, HoldConsideration},
		tokens::{
			imbalance::ResolveTo, pay::PayFromAccount, Imbalance, UnityAssetBalanceConversion,
		},
		ConstU32, Contains, Currency, EitherOf, EitherOfDiverse, EqualPrivilegeOnly, InsideBoth,
		InstanceFilter, LinearStoragePrice, OnUnbalanced,
	},
	weights::{constants::RocksDbWeight, ConstantMultiplier},
	PalletId,
};
use frame_system::{limits::BlockLength, EnsureRoot, EnsureRootWithSuccess, EnsureWithSuccess};
use pallet_election_provider_multi_phase::{GeometricDepositBase, SolutionAccuracyOf};
use pallet_identity::legacy::IdentityInfo;
use pallet_transaction_payment::{FungibleAdapter, Multiplier, TargetedFeeAdjustment};
use pallet_treasury::TreasuryAccountId;
use pallet_tx_pause::RuntimeCallNameOf;
use sp_core::{ConstU64, RuntimeDebug};
use sp_runtime::{
	generic::Era,
	traits::{self, BlakeTwo256, Bounded, Convert, IdentityLookup, OpaqueKeys},
	FixedPointNumber, FixedU128, Perbill, Permill, Perquintill,
};
use sp_staking::EraIndex;

pub type NegativeImbalance<T> = <pallet_balances::Pallet<T> as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

impl pallet_sudo::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

impl pallet_mandate::Config for Runtime {
	type ApprovedOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 5, 7>,
	>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_mandate::WeightInfo<Runtime>;
}

parameter_types! {
	pub const BridgePalletId: PalletId = PalletId(*b"avl/brdg");
}

impl pallet_vector::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_vector::WeightInfo<Runtime>;
	type TimeProvider = pallet_timestamp::Pallet<Runtime>;
	type Currency = Balances;
	type MessageMappingStorageIndex = ConstU64<1>;
	type PalletId = BridgePalletId;
	type AvailDomain = ConstU32<1>;
}

parameter_types! {
	pub const BasicDeposit: Balance = 100 * AVAIL;
	pub const ByteDeposit: Balance = constants::currency::deposit(0,1);
	pub const SubAccountDeposit: Balance = 2 * AVAIL;
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	/// The amount held on deposit for a registered identity.
	type BasicDeposit = BasicDeposit;
	/// The amount held on deposit per additional bytes in additional fields for a registered identity
	type ByteDeposit = ByteDeposit;
	type Currency = Balances;
	type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
	/// The origin which may forcibly set or remove a name. Root can always do this.
	type ForceOrigin = EnsureRoot<AccountId>;
	/// Maximum number of registrars allowed in the system.
	type MaxRegistrars = MaxRegistrars;
	/// The maximum number of sub-accounts allowed per identified account.
	type MaxSubAccounts = MaxSubAccounts;
	type MaxSuffixLength = ConstU32<7>;
	type MaxUsernameLength = ConstU32<32>;
	type OffchainSignature = Signature;
	type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
	/// The origin which may add or remove registrars. Root can always do this.
	type RegistrarOrigin = EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type SigningPublicKey = <Signature as traits::Verify>::Signer;
	type Slashed = Treasury;
	/// The amount held on deposit for a registered subaccount.
	type SubAccountDeposit = SubAccountDeposit;
	type UsernameAuthorityOrigin = EnsureRoot<Self::AccountId>;
	type WeightInfo = weights::pallet_identity::WeightInfo<Runtime>;
}

impl da_control::Config for Runtime {
	type BlockLenProposalId = u32;
	type MaxAppDataLength = constants::da::MaxAppDataLength;
	type MaxAppKeyLength = constants::da::MaxAppKeyLength;
	type MaxBlockCols = constants::da::MaxBlockCols;
	type MaxBlockRows = constants::da::MaxBlockRows;
	type MinBlockCols = constants::da::MinBlockCols;
	type MinBlockRows = constants::da::MinBlockRows;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_dactr::WeightInfo<Runtime>;
}

impl pallet_offences::Config for Runtime {
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
	type RuntimeEvent = RuntimeEvent;
}

impl pallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = constants::MaxAuthorities;
}

parameter_types! {
	pub const FusionPalletId: PalletId = PalletId(*b"avl/fusi");
	pub const MaxCurrencyNameLength: u32 = 32;
	pub const MaxMembersPerPool: u32 = 100_000;
	pub const MaxTargets: u32 = 16;
	pub const MaxUnbonding: u32 = 8;
	pub const HistoryDepth: u32 = 84;
	pub const MaxSlashesPerPool: u32 = 100; // If we have 100 slashes during 27 eras, we have bigger problems
	pub const MaxPoolsPerValidator: u32 = 100;
}
impl pallet_fusion::Config for Runtime {
	type Currency = Balances;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type ApprovedOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 5, 7>,
	>;
	type PalletId = FusionPalletId;
	type MaxCurrencyNameLength = MaxCurrencyNameLength;
	type MaxMembersPerPool = MaxMembersPerPool;
	type MaxTargets = MaxTargets;
	type MaxUnbonding = MaxUnbonding;
	type MaxSlashesPerPool = MaxSlashesPerPool;
	type MaxPoolsPerValidator = MaxPoolsPerValidator;
	type BondingDuration = constants::staking::FusionBondingDuration;
	type SlashDeferDuration = constants::staking::SlashDeferDuration;
	type RewardRemainder = Treasury;
	type HistoryDepth = HistoryDepth;
	type StakingFusionDataProvider = Self;
	type PoolAccountProvider = ();
	type WeightInfo = weights::pallet_fusion::WeightInfo<Runtime>;
}

impl pallet_fusion::StakingFusionDataProvider<Self> for Runtime {
	fn active_era() -> EraIndex {
		pallet_staking::Pallet::<Self>::active_era()
			.map(|era_info| era_info.index)
			.unwrap_or(0)
	}
	fn current_era() -> EraIndex {
		pallet_staking::Pallet::<Self>::current_era().unwrap_or_default()
	}
	fn is_valid_validator(account: &AccountId) -> bool {
		pallet_staking::Validators::<Self>::contains_key(account)
			&& !pallet_staking::Validators::<Self>::get(account).blocked
	}
	fn has_earned_era_points(era: EraIndex, accounts: &Vec<AccountId>) -> bool {
		let era_points = pallet_staking::ErasRewardPoints::<Self>::get(era).individual;
		accounts
			.iter()
			.any(|account| era_points.contains_key(account))
	}
	fn unapplied_slashes(era: EraIndex) -> Vec<UnappliedSlash<AccountId, Balance>> {
		pallet_staking::UnappliedSlashes::<Self>::get(era)
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add_dummy_validator(account: AccountId) {
		pallet_session::Validators::<Self>::mutate(|validators| {
			validators.push(account.clone());
		});

		let default_prefs = pallet_staking::ValidatorPrefs {
			commission: Perbill::from_percent(0),
			blocked: false,
		};
		pallet_staking::Validators::<Self>::insert(account.clone(), default_prefs);
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add_dummy_era_points(validator: AccountId, era: EraIndex) {
		pallet_staking::ErasRewardPoints::<Self>::mutate(era, |reward_points| {
			let points = reward_points
				.individual
				.entry(validator.clone())
				.or_insert(0);
			*points += 100;
		});
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn set_dummy_active_era(era: EraIndex) {
		pallet_staking::ActiveEra::<Self>::mutate(|active_era| {
			*active_era = Some(pallet_staking::ActiveEraInfo {
				index: era,
				start: None,
			});
		});
	}
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(50);
	pub const ProposalBondMinimum: Balance = 100 * AVAIL;
	pub const SpendPeriod: BlockNumber = DAYS;
	pub const Burn: Permill = Permill::from_percent(0); // Not burning any funds for now
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const MaxApprovals: u32 = 100;
}

parameter_types! {
	pub const WeightFee: Balance = 10 * PICO_AVAIL;
	pub const TransactionByteFee: Balance = 100 * NANO_AVAIL; // 100 nanoAVAIL
	pub const OperationalFeeMultiplier: u8 = 5u8;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(50); // target_utilization 50%
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000); // 0.000001
	// pub LenAdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(4, 1000); // 0.004 to make fee 4x in one epoch on a fully congested network
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
	pub MinLenMultiplier: Multiplier = Multiplier::from_u32(1);
	pub MaximumMultiplier: Multiplier = Bounded::max_value();
}

impl pallet_transaction_payment::Config for Runtime {
	type FeeMultiplierUpdate = TargetedFeeAdjustment<
		Self,
		TargetBlockFullness,
		AdjustmentVariable,
		MinimumMultiplier,
		MaximumMultiplier,
	>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type OnChargeTransaction = FungibleAdapter<Balances, DealWithFees<Runtime>>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type RuntimeEvent = RuntimeEvent;
	type WeightToFee = ConstantMultiplier<Balance, WeightFee>; // 1 weight = 10 picoAVAIL -> second_price = 10 AVAIL
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	type MinimumPeriod = MinimumPeriod;
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type EventHandler = (Staking, ImOnline);
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
}

impl pallet_session::Config for Runtime {
	type Keys = SessionKeys;
	type NextSessionRotation = Babe;
	type RuntimeEvent = RuntimeEvent;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type ShouldEndSession = Babe;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

/// Logic for the author to get a portion of fees.
pub struct Author<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>> for Author<R>
where
	R: pallet_balances::Config + pallet_authorship::Config,
	<R as frame_system::Config>::AccountId: From<AccountId>,
	<R as frame_system::Config>::AccountId: Into<AccountId>,
{
	fn on_nonzero_unbalanced(
		amount: Credit<<R as frame_system::Config>::AccountId, pallet_balances::Pallet<R>>,
	) {
		if let Some(author) = <pallet_authorship::Pallet<R>>::author() {
			let _ = <pallet_balances::Pallet<R>>::resolve(&author, amount);
		}
	}
}

pub struct DealWithFees<R>(core::marker::PhantomData<R>);
impl<R> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>> for DealWithFees<R>
where
	R: pallet_balances::Config + pallet_authorship::Config + pallet_treasury::Config,
	<R as frame_system::Config>::AccountId: From<AccountId>,
	<R as frame_system::Config>::AccountId: Into<AccountId>,
{
	fn on_unbalanceds<B>(
		mut fees_then_tips: impl Iterator<Item = Credit<R::AccountId, pallet_balances::Pallet<R>>>,
	) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees, 20% to author, 80% to treasury
			let mut split = fees.ration(80, 20);
			if let Some(tips) = fees_then_tips.next() {
				// for tips, if any, 100% to author
				tips.merge_into(&mut split.1);
			}
			ResolveTo::<TreasuryAccountId<R>, pallet_balances::Pallet<R>>::on_unbalanced(split.0);
			<Author<R> as OnUnbalanced<_>>::on_unbalanced(split.1);
		}
	}
}

impl pallet_utility::Config for Runtime {
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

parameter_types! {
	pub const DepositBase: Balance = 2 * AVAIL;
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = 5 * CENTS;
}

impl pallet_multisig::Config for Runtime {
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = ConstU32<100>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		constants::system::RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
	type MaxScheduledPerBlock = ConstU32<512>;
	type MaximumWeight = MaximumSchedulerWeight;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type PalletsOrigin = OriginCaller;
	type Preimages = Preimage;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pallet_scheduler::WeightInfo<Runtime>;
}

impl pallet_preimage::Config for Runtime {
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		constants::preimage::PreimageHoldReason,
		LinearStoragePrice<
			constants::preimage::PreimageBaseDeposit,
			constants::preimage::PreimageByteDeposit,
			Balance,
		>,
	>;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
}

impl pallet_babe::Config for Runtime {
	type DisabledValidators = Session;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type EpochDuration = constants::time::EpochDuration;
	type EquivocationReportSystem = pallet_babe::EquivocationReportSystem<
		Self,
		Offences,
		Historical,
		constants::babe::ReportLongevity,
	>;
	type ExpectedBlockTime = constants::time::ExpectedBlockTime;
	type KeyOwnerProof = sp_session::MembershipProof;
	type MaxAuthorities = constants::MaxAuthorities;
	type MaxNominators = constants::staking::MaxNominators;
	type WeightInfo = ();
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = constants::indices::IndexDeposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_indices::WeightInfo<Runtime>;
}

impl pallet_balances::Config for Runtime {
	type AccountStore = frame_system::Pallet<Runtime>;
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = constants::balances::ExistentialDeposit;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<2>;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = ReserveIdentifier;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type MaxKeys = ConstU32<10_000>;
	type MaxPeerInHeartbeats = ConstU32<10_000>;
	type NextSessionRotation = Babe;
	type ReportUnresponsiveness = Offences;
	type RuntimeEvent = RuntimeEvent;
	type UnsignedPriority = constants::im::ImOnlineUnsignedPriority;
	type ValidatorSet = Historical;
	type WeightInfo = weights::pallet_im_online::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxSetIdSessionEntries: u32 = constants::staking::BondingDuration::get() * constants::staking::SessionsPerEra::get();
}

impl pallet_grandpa::Config for Runtime {
	type EquivocationReportSystem = pallet_grandpa::EquivocationReportSystem<
		Self,
		Offences,
		Historical,
		constants::babe::ReportLongevity,
	>;
	type KeyOwnerProof = sp_session::MembershipProof;
	type MaxAuthorities = constants::MaxAuthorities;
	type MaxNominators = constants::staking::MaxNominators;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}

parameter_types! {
	pub const TreasuryMotionDuration: BlockNumber = prod_or_fast!(5 * DAYS, 5 * MINUTES);
}

pub type TreasuryCollective = pallet_collective::Instance1;

impl pallet_collective::Config<TreasuryCollective> for Runtime {
	type DefaultVote = pallet_collective::MoreThanMajorityThenPrimeDefaultVote;
	type MaxMembers = ConstU32<100>;
	type MaxProposalWeight = constants::council::MaxProposalWeight;
	type MaxProposals = ConstU32<100>;
	type MotionDuration = TreasuryMotionDuration;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type WeightInfo = weights::pallet_collective::WeightInfo<Runtime>;
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = prod_or_fast!(5 * DAYS, 5 * MINUTES);
}
pub type TechnicalMaxMembers = ConstU32<100>;

pub type TechnicalCollective = pallet_collective::Instance2;

impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type DefaultVote = pallet_collective::MoreThanMajorityThenPrimeDefaultVote;
	type MaxMembers = TechnicalMaxMembers;
	type MaxProposalWeight = constants::council::MaxProposalWeight;
	type MaxProposals = ConstU32<100>;
	type MotionDuration = TechnicalMotionDuration;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = constants::staking::MinerMaxLength;
	type MaxVotesPerVoter =
    <<Self as pallet_election_provider_multi_phase::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter;
	type MaxWeight = constants::staking::MinerMaxWeight;
	type MaxWinners = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
	type Solution = constants::staking::NposSolution16;

	// The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
	// weight estimate function is wired to this call's weight.
	fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
		<
        <Self as pallet_election_provider_multi_phase::Config>::WeightInfo
        as
        pallet_election_provider_multi_phase::WeightInfo
        >::submit_unsigned(v, t, a, d)
	}
}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
	type BetterSignedThreshold = ();
	type Currency = Balances;
	// nothing to do upon rewards
	type DataProvider = Staking;
	type ElectionBounds = constants::staking::ElectionBoundsMultiPhase;
	type EstimateCallFee = TransactionPayment;
	type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type MaxWinners = constants::staking::MaxActiveValidators;
	type MinerConfig = Self;
	type MinerTxPriority = constants::staking::MultiPhaseUnsignedPriority;
	type OffchainRepeat = constants::staking::OffchainRepeat;
	// burn slashes
	type RewardHandler = ();
	type RuntimeEvent = RuntimeEvent;
	type SignedDepositBase = GeometricDepositBase<
		Balance,
		constants::staking::SignedFixedDeposit,
		constants::staking::SignedDepositIncreaseFactor,
	>;
	type SignedDepositByte = constants::staking::SignedDepositByte;
	type SignedDepositWeight = ();
	type SignedMaxRefunds = constants::staking::SignedMaxRefunds;
	type SignedMaxSubmissions = constants::staking::SignedMaxSubmissions;
	type SignedMaxWeight = constants::staking::MinerMaxWeight;
	type SignedPhase = constants::staking::SignedPhase;
	type SignedRewardBase = constants::staking::SignedRewardBase;
	type SlashHandler = ();
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, OffchainRandomBalancing>;
	type UnsignedPhase = constants::staking::UnsignedPhase;
	type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Self>;
}

pub struct StakingBenchmarkingConfig;

/// A reasonable benchmarking config for staking pallet.
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxNominators = ConstU32<1024>;
	type MaxValidators = ConstU32<1024>;
}

impl pallet_staking::Config for Runtime {
	/// A super-majority of the council can cancel the slash.
	type AdminOrigin = EnsureRoot<AccountId>;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type BondingDuration = constants::staking::BondingDuration;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
	type ElectionProvider = ElectionProviderMultiPhase;
	type EraPayout = pallet_staking::ConvertCurve<constants::staking::RewardCurve>;
	type EventListeners = NominationPools;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type HistoryDepth = constants::staking::HistoryDepth;
	type MaxControllersInDeprecationBatch = constants::staking::MaxControllersInDeprecationBatch;
	type MaxExposurePageSize = constants::staking::MaxExposurePageSize;
	type MaxUnlockingChunks = constants::staking::MaxUnlockingChunks;
	type NextNewSession = Session;
	type NominationsQuota =
		pallet_staking::FixedNominationsQuota<{ constants::staking::MaxNominations::get() }>;
	type OffendingValidatorsThreshold = constants::staking::OffendingValidatorsThreshold;
	type Reward = ();
	type RewardRemainder = Treasury;
	type RuntimeEvent = RuntimeEvent;
	type SessionInterface = Self;
	// rewards are minted from the void
	type SessionsPerEra = constants::staking::SessionsPerEra;
	// send the slashed funds to the treasury.
	type Slash = Treasury;
	type SlashDeferDuration = constants::staking::SlashDeferDuration;
	// This a placeholder, to be introduced in the next PR as an instance of bags-list
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type UnixTime = Timestamp;
	type VoterList = VoterList;
	type WeightInfo = weights::pallet_staking::WeightInfo<Runtime>;
	type FusionExt = Fusion;
}

/// The numbers configured here could always be more than the maximum limits of staking pallet
/// to ensure election snapshot will not run out of memory. For now, we set them to smaller values
/// since the staking is bounded and the weight pipeline takes hours for this single pallet.
pub struct ElectionProviderBenchmarkConfig;

impl pallet_election_provider_multi_phase::BenchmarkingConfig for ElectionProviderBenchmarkConfig {
	const ACTIVE_VOTERS: [u32; 2] = [500, 800];
	const DESIRED_TARGETS: [u32; 2] = [200, 400];
	const MAXIMUM_TARGETS: u32 = 300;
	const MINER_MAXIMUM_VOTERS: u32 = 1000;
	const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
	const TARGETS: [u32; 2] = [500, 1000];
	const VOTERS: [u32; 2] = [1000, 2000];
}

/// Maximum number of iterations for balancing that will be executed in the embedded OCW
/// miner of election provider multi phase.
pub const MINER_MAX_ITERATIONS: u32 = 10;

/// A source of random balance for NposSolver, which is meant to be run by the OCW election miner.
pub struct OffchainRandomBalancing;

impl Get<Option<BalancingConfig>> for OffchainRandomBalancing {
	fn get() -> Option<BalancingConfig> {
		use sp_runtime::traits::TrailingZeroInput;
		let iterations = match MINER_MAX_ITERATIONS {
			0 => 0,
			max => {
				let seed = sp_io::offchain::random_seed();
				let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
					.expect("input is padded with zeroes; qed")
					% max.saturating_add(1);
				random as usize
			},
		};

		let config = BalancingConfig {
			iterations,
			tolerance: 0,
		};
		Some(config)
	}
}

pub struct OnChainSeqPhragmen;

impl onchain::Config for OnChainSeqPhragmen {
	type Bounds = constants::staking::ElectionBoundsOnChain;
	type DataProvider = <Runtime as pallet_election_provider_multi_phase::Config>::DataProvider;
	type MaxWinners = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Runtime>,
	>;
	type System = Runtime;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &voter_bags::THRESHOLDS;
}

type VoterBagsListInstance = pallet_bags_list::Instance1;

impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type BagThresholds = BagThresholds;
	type RuntimeEvent = RuntimeEvent;
	type Score = VoteWeight;
	/// The voter bags-list is loosely kept up to date, and the real source of truth for the score
	/// of each node is the staking pallet.
	type ScoreProvider = Staking;
	type WeightInfo = pallet_bags_list::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const PostUnbondPoolsWindow: u32 = 4;
	pub const NominationPoolsPalletId: PalletId = PalletId(*b"py/nopls");
	pub const MaxPointsToBalance: u8 = 10;
}

pub struct BalanceToU256;

impl Convert<Balance, sp_core::U256> for BalanceToU256 {
	fn convert(balance: Balance) -> sp_core::U256 {
		sp_core::U256::from(balance)
	}
}

pub struct U256ToBalance;

impl Convert<sp_core::U256, Balance> for U256ToBalance {
	fn convert(n: sp_core::U256) -> Balance {
		n.try_into().unwrap_or(Balance::max_value())
	}
}

impl pallet_nomination_pools::Config for Runtime {
	type BalanceToU256 = BalanceToU256;
	type Currency = Balances;
	type MaxMetadataLen = ConstU32<256>;
	type MaxPointsToBalance = MaxPointsToBalance;
	type MaxUnbonding = ConstU32<8>;
	type PalletId = NominationPoolsPalletId;
	type PostUnbondingPoolsWindow = PostUnbondPoolsWindow;
	type RewardCounter = FixedU128;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type Staking = Staking;
	type U256ToBalance = U256ToBalance;
	type WeightInfo = ();
}

parameter_types! {
	pub const SpendPayoutPeriod: BlockNumber = 30 * DAYS;
	pub TreasuryAccount: AccountId = Treasury::account_id();
	pub const MaxBalance: Balance = Balance::max_value();
	pub const MaxTreasurySpend: Balance = 10_000_000 * AVAIL; // 10 Million AVAILs
}

pub type TreasurySpender =
	pallet_collective::EnsureProportionAtLeast<AccountId, TreasuryCollective, 5, 7>;

impl pallet_treasury::Config for Runtime {
	type ApproveOrigin = EnsureRoot<AccountId>;
	type Burn = Burn;
	type BurnDestination = ();
	type Currency = Balances;
	type MaxApprovals = MaxApprovals;
	type OnSlash = Treasury;
	type PalletId = TreasuryPalletId;
	type ProposalBond = ProposalBond;
	type ProposalBondMaximum = ();
	type ProposalBondMinimum = ProposalBondMinimum;
	type RejectOrigin = EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type SpendFunds = ();
	type SpendOrigin = EitherOf<
		EnsureRootWithSuccess<AccountId, MaxBalance>,
		EnsureWithSuccess<TreasurySpender, AccountId, MaxTreasurySpend>,
	>;
	type SpendPeriod = SpendPeriod;
	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
	type AssetKind = ();
	type Beneficiary = AccountId;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type BalanceConverter = UnityAssetBalanceConversion;
	type PayoutPeriod = SpendPayoutPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_mmr::Config for Runtime {
	type Hashing = <Runtime as frame_system::Config>::Hashing;
	type LeafData = pallet_mmr::ParentNumberAndHash<Self>;
	type OnNewRoot = ();
	type WeightInfo = ();

	const INDEXING_PREFIX: &'static [u8] = b"mmr";
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	Any,
	NonTransfer,
	Governance,
	Staking,
	IdentityJudgement,
	NominationPools,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => !matches!(
				c,
				RuntimeCall::Balances(..)
					| RuntimeCall::Indices(pallet_indices::Call::transfer { .. })
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::TechnicalCommittee(..)
					| RuntimeCall::Treasury(..)
					| RuntimeCall::TreasuryCommittee(..)
			),
			ProxyType::Staking => matches!(
				c,
				RuntimeCall::Session(..)
					| RuntimeCall::Staking(..)
					| RuntimeCall::Utility(..)
					| RuntimeCall::VoterList(..)
					| RuntimeCall::NominationPools(..)
			),
			ProxyType::IdentityJudgement => matches!(
				c,
				RuntimeCall::Identity(pallet_identity::Call::provide_judgement { .. })
					| RuntimeCall::Utility(..)
			),
			ProxyType::NominationPools => matches!(
				c,
				RuntimeCall::NominationPools(..) | RuntimeCall::Utility(..)
			),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = 10 * AVAIL;
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = 3 * AVAIL;
	pub const AnnouncementDepositBase: Balance = 10 * AVAIL;
	pub const AnnouncementDepositFactor: Balance = 5 * AVAIL;
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = ConstU32<32>;
	type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
	type MaxPending = ConstU32<32>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

/// Calls that cannot be paused by the tx-pause pallet.
pub struct TxPauseWhitelistedCalls;
/// All calls are pauseable.
impl Contains<RuntimeCallNameOf<Runtime>> for TxPauseWhitelistedCalls {
	fn contains(_full_name: &RuntimeCallNameOf<Runtime>) -> bool {
		false
		// match (full_name.0.as_slice(), full_name.1.as_slice()) {
		// 	(b"Balances", b"transfer_keep_alive") => true,
		// 	_ => false,
		// }
	}
}

impl pallet_tx_pause::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PauseOrigin = EnsureRoot<AccountId>;
	type UnpauseOrigin = EnsureRoot<AccountId>;
	type WhitelistedCalls = TxPauseWhitelistedCalls;
	type MaxNameLen = ConstU32<256>;
	type WeightInfo = weights::pallet_tx_pause::WeightInfo<Runtime>;
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
}

// Configure FRAME pallets to include in runtime.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// The maximum length of a block (in bytes).
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = InsideBoth<Everything, TxPause>;
	/// The Block type used by the runtime
	type Block = Block;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	type BlockLength = RuntimeBlockLength;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = constants::system::RuntimeBlockWeights;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header builder type.
	type HeaderExtensionBuilder =
		frame_system::native::hosted_header_builder::da::HeaderExtensionBuilder<Runtime>;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = Indices;
	type MaxConsumers = constants::system::MaxConsumers;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Index;
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// What to do if a new account is created.
	type OnNewAccount = ();
	// What to do if runtime code change: Default behaviour.
	type OnSetCode = ();
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// Randomness
	type Randomness = pallet_babe::RandomnessFromOneEpochAgo<Runtime>;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeTask = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = constants::system::SS58Prefix;
	/// Data Root
	type HeaderExtensionDataFilter = Runtime;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	/// Version of the runtime.
	type Version = Version;

	type Extrinsic = UncheckedExtrinsic;
	type Header = Header;
	type MaxDiffAppIdPerBlock = ConstU32<1_024>;
	type MaxTxPerAppIdPerBlock = ConstU32<8_192>;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		public: <Signature as traits::Verify>::Signer,
		account: AccountId,
		nonce: Index,
	) -> Option<(
		RuntimeCall,
		<UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload,
	)> {
		use sp_runtime::{traits::StaticLookup, SaturatedConversion as _};

		let tip = 0;
		// take the biggest period possible.
		let period = BlockHashCount::get()
			.checked_next_power_of_two()
			.map(|c| c / 2)
			.unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let era = Era::mortal(period, current_block);
		let extra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(era),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			da_control::CheckAppId::<Runtime>::from(AppId(0)),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = Indices::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature, extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}
