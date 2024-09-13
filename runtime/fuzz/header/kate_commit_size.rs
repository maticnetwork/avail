#![no_main]

use anyhow::{anyhow, Result};
use arbitrary::{Arbitrary, Result as AResult, Unstructured};
use rand::{distributions::Standard, thread_rng, Rng};
use std::{
	cmp::{max, min},
	mem::size_of,
	num::NonZeroUsize,
	panic,
};

use avail_core::block::header::HeaderExtension;
use da_control::{pallet::Call as DaControlCall, AppDataFor, CheckAppId};
use da_runtime::{
	AppId, Executive, Header, Runtime, RuntimeCall, RuntimeGenesisConfig, SignedExtra,
	SignedPayload, Timestamp, UncheckedExtrinsic, AVAIL,
};

use codec::Encode;
use frame_support::pallet_prelude::{
	InvalidTransaction as InvalidTx, TransactionValidityError as TxErr,
};
use frame_system::{
	CheckEra, CheckGenesis, CheckNonZeroSender, CheckNonce, CheckSpecVersion, CheckTxVersion,
	CheckWeight,
};
use pallet_transaction_payment::ChargeTransactionPayment;
use sp_core::{Pair, H256};
use sp_io::TestExternalities;
use sp_keyring::AccountKeyring::Alice;
use sp_runtime::{generic::Era, BuildStorage, Digest, MultiAddress, MultiSignature};

fn runtime_ext(total_app_ids: NonZeroUsize) -> TestExternalities {
	let alice = Alice.to_account_id();
	let mut t = RuntimeGenesisConfig::default()
		.system
		.build_storage()
		.unwrap();

	pallet_babe::GenesisConfig::<Runtime> {
		epoch_config: Some(da_runtime::constants::babe::GENESIS_EPOCH_CONFIG),
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	// Alice has all the money
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(alice.clone(), 100_000_000 * AVAIL)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	// Generate valid app IDs
	da_control::GenesisConfig::<Runtime> {
		app_keys: (0..=total_app_ids.get())
			.map(|i| (b"".to_vec(), (alice.clone(), i as u32)))
			.collect::<Vec<_>>(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}

fn submit_data(
	nonce: u32,
	app_id: AppId,
	data: Vec<u8>,
) -> Result<(UncheckedExtrinsic, RuntimeCall)> {
	let data = AppDataFor::<Runtime>::try_from(data).map_err(|_| anyhow!("Data too long"))?;
	let call = RuntimeCall::DataAvailability(DaControlCall::<Runtime>::submit_data { data });

	let extra: SignedExtra = (
		CheckNonZeroSender::<Runtime>::new(),
		CheckSpecVersion::<Runtime>::new(),
		CheckTxVersion::<Runtime>::new(),
		CheckGenesis::<Runtime>::new(),
		CheckEra::<Runtime>::from(Era::Immortal),
		CheckNonce::<Runtime>::from(nonce),
		CheckWeight::<Runtime>::new(),
		ChargeTransactionPayment::<Runtime>::from(0),
		CheckAppId::<Runtime>::from(app_id),
	);
	let payload =
		SignedPayload::new(call, extra).map_err(|e| anyhow!("Failed to create payload: {e:?}"))?;
	let enc_payload = payload.encode();
	let signature = MultiSignature::from(Alice.pair().sign(&enc_payload));
	let (call, extra, _signed) = payload.deconstruct();
	let signer = MultiAddress::from(Alice.to_account_id());

	let tx = UncheckedExtrinsic::new_signed(call.clone(), signer, signature, extra);
	Ok((tx, call))
}

fn header() -> Header {
	let extrinsic_root = H256::default();
	let state_root = H256::default();
	let parent_hash = [69u8; 32].into();
	let digest = Digest::default();

	let ext = HeaderExtension::default();
	Header::new(1, extrinsic_root, state_root, parent_hash, digest, ext)
}

const MAX_IDS: usize = 1_024 * 1_024;
const MAX_TXS: usize = 8 * 1_024;

#[derive(Clone, Debug)]
struct TestData {
	data_lens: Vec<NonZeroUsize>,
	total_app_id: NonZeroUsize,
}

impl<'a> Arbitrary<'a> for TestData {
	fn arbitrary(u: &mut Unstructured<'a>) -> AResult<Self> {
		let max_app_data_len: u8 = 128;
		// Get an iterator of arbitrary `T`s.
		let pre_total_app_id = usize::arbitrary(u).unwrap_or(1);
		let total_app_id = max(1, min(pre_total_app_id, MAX_IDS));
		let total_app_id = unsafe { NonZeroUsize::new_unchecked(total_app_id) };

		let len = u.arbitrary_len::<usize>().unwrap_or(1);
		let len = max(1, min(len, MAX_TXS));
		let mut data_lens = Vec::with_capacity(len);

		/*
		let iter = u.arbitrary_iter::<usize>()?;
		for len_res in iter {
			let len = len_res?;
			let len = max(1, min(len, max_app_data_len as usize));
			data_lens.push(unsafe { NonZeroUsize::new_unchecked(len) });
		}*/
		for _ in 0..len {
			let len = u8::arbitrary(u).unwrap_or(128);
			let len = max(1, min(len, max_app_data_len));
			data_lens.push(unsafe { NonZeroUsize::new_unchecked(len as usize) });
		}

		Ok(TestData {
			data_lens,
			total_app_id,
		})
	}

	fn size_hint(_depth: usize) -> (usize, Option<usize>) {
		let lower_bound = size_of::<usize>() + 30 * size_of::<usize>();
		let upper_bound = size_of::<usize>() + MAX_TXS * size_of::<usize>();

		(lower_bound, Some(upper_bound))
	}
}

fn kate_commit_size_fuzzer(total_app_id: NonZeroUsize, data_lens: Vec<NonZeroUsize>) {
	eprintln!(
		"Running with {} ids and {} data lengths",
		total_app_id.get(),
		data_lens.len()
	);

	runtime_ext(total_app_id).execute_with(|| {
		let txs = data_lens
			.into_iter()
			.take(MAX_TXS)
			.enumerate()
			.map(|(i, data_len)| {
				let nonce = i as u32;
				let app_id = AppId(nonce % total_app_id.get() as u32);
				(nonce, app_id, data_len)
			})
			.collect::<Vec<_>>();
		let total_txs = txs.len();

		Executive::initialize_block(&header());
		Timestamp::set_timestamp(0);

		let mut total_ok_txs = 0u32;
		for (i, (nonce, app_id, data_len)) in txs.into_iter().enumerate() {
			let data = thread_rng()
				.sample_iter(&Standard)
				.take(data_len.get())
				.collect::<Vec<_>>();
			let (tx, _call) = submit_data(nonce, app_id, data).unwrap();
			let (_address, _signature, _extra) = tx.signature.as_ref().unwrap();

			match Executive::apply_extrinsic(tx) {
				Ok(_) => total_ok_txs += 1,
				Err(e) => match e {
					TxErr::Invalid(InvalidTx::ExhaustsResources) => break,
					_ => panic!("Failed to apply extrinsic at index {i}: {e:?}"),
				},
			}
		}
		eprintln!("Applied {total_ok_txs} out of {total_txs} txs");
		Executive::finalize_block();
	});
}

#[cfg(feature = "use_afl")]
pub fn main() {
	afl::fuzz!(|data: TestData| {
		kate_commit_size_fuzzer(data.total_app_id, data.data_lens);
	});
}

#[cfg(feature = "use_fuzzer")]
libfuzzer_sys::fuzz_target!(|data: TestData| {
	if data.total_app_id.get() <= 512 || data.total_app_id.get() > 4 * 1_024 {
		return;
	}
	if data.data_lens.len() <= 256 || data.data_lens.len() > 3 * 1_024 {
		return;
	}

	kate_commit_size_fuzzer(data.total_app_id, data.data_lens);
});
