// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use frame_support::{
	construct_runtime, derive_impl, dispatch::DispatchClass, parameter_types, traits::ConstU32,
	weights::Weight,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	mocking::{MockDaBlock, MockUncheckedExtrinsic},
	native::hosted_header_builder::da::HeaderExtensionBuilder,
	test_utils::TestRandomness,
};
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, Perbill,
};

#[frame_support::pallet]
mod module {
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {
		Complex(Vec<u8>, u32, u16, u128),
	}
}

type UncheckedExtrinsic = MockUncheckedExtrinsic<Runtime>;
type Block = MockDaBlock<Runtime>;

construct_runtime!(
	pub struct Runtime
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Module: module::{Pallet, Event},
	}
);

const NORMAL_DISPATCH_RATIO_PCT: Perbill = Perbill::from_percent(75);
const MAX_BLOCK_WEIGHT: Weight = Weight::from_parts(1024, u64::MAX);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(Weight::from_parts(10, 0))
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = Weight::from_parts(5, 0);
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO_PCT * MAX_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.base_extrinsic = Weight::from_parts(10, 0);
			weights.max_total = Some(MAX_BLOCK_WEIGHT);
			weights.reserved = Some(
				MAX_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO_PCT * MAX_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(Perbill::from_percent(0))
		.build_or_panic();
	pub RuntimeBlockLength: BlockLength = BlockLength::max_with_normal_ratio(1024, NORMAL_DISPATCH_RATIO_PCT);
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU32<10>;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type AccountData = u32;

	type HeaderExtensionBuilder = HeaderExtensionBuilder<Runtime>;
	type Randomness = TestRandomness<Runtime>;
	type HeaderExtensionDataFilter = ();
	type Extrinsic = UncheckedExtrinsic;
	type MaxDiffAppIdPerBlock = ConstU32<1_024>;
	type MaxTxPerAppIdPerBlock = ConstU32<8_192>;
}

impl module::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap()
		.into()
}

fn deposit_events(n: usize) {
	let mut t = new_test_ext();
	t.execute_with(|| {
		for _ in 0..n {
			module::Pallet::<Runtime>::deposit_event(module::Event::Complex(
				vec![1, 2, 3],
				2,
				3,
				899,
			));
		}
	});
}

fn sr_system_benchmark(c: &mut Criterion) {
	c.bench_function("deposit 100 events", |b| {
		b.iter(|| deposit_events(black_box(100)))
	});
}

criterion_group!(benches, sr_system_benchmark);
criterion_main!(benches);
