use super::{alice_nonce, allow_concurrency, local_connection};

use avail_core::AppId;
use avail_subxt::{
	avail::{GDataProof, GRow, Rows, TxInBlock, TxProgress},
	primitives::Cell,
	submit::submit_data_with_nonce,
	tx, RpcParams,
};
use subxt_signer::sr25519::dev;

use futures::stream::{FuturesOrdered, TryStreamExt as _};
use rand::{
	distributions::{Distribution, Standard},
	thread_rng, Rng,
};
use std::{collections::HashSet, sync::atomic::Ordering::Relaxed};
use test_log::test;
use tracing::trace;

fn data(count: usize, len: usize) -> Vec<Vec<u8>> {
	let mut rng = thread_rng();
	(0..count)
		.map(|_| {
			Standard
				.sample_iter(&mut rng)
				.take(len)
				.collect::<Vec<u8>>()
		})
		.collect()
}

/// This example submits an Avail data extrinsic, then retrieves the block containing the
/// extrinsic and matches the data.
#[test(tokio::test)]
async fn submit_data() -> anyhow::Result<()> {
	let _cg = allow_concurrency("submit_data").await;
	let client = local_connection().await?;

	let alice = dev::alice();
	let sub_datas = data(5, 1_024);
	let mut try_count = 0;

	let block_hash = loop {
		let nonce = alice_nonce()
			.await
			.fetch_add(sub_datas.len() as u64, Relaxed);
		trace!("Submitting data to network");
		let txs = sub_datas
			.iter()
			.enumerate()
			.map(|(idx, data)| {
				submit_data_with_nonce(
					&client,
					&alice,
					data.as_slice(),
					AppId(1),
					nonce + idx as u64,
				)
			})
			.collect::<FuturesOrdered<_>>()
			.try_collect::<Vec<TxProgress>>()
			.await?;

		trace!("Waiting until data submitted is finalized");
		let in_blocks = txs
			.into_iter()
			.map(tx::in_finalized)
			.collect::<FuturesOrdered<_>>()
			.try_collect::<Vec<TxInBlock>>()
			.await?;

		let hashes = in_blocks
			.iter()
			.map(|p| p.block_hash())
			.collect::<HashSet<_>>();
		trace!("Submitted data in blocks: {hashes:?}");
		if hashes.len() == 1 {
			break hashes.into_iter().next().unwrap();
		}
		try_count += 1;
		if try_count > 5 {
			panic!("Could not submit data to the same block");
		}
	};

	/*
	let extrinsics = client.blocks().at(hash).await?.extrinsics().await?;
	let submit_call = extrinsics.find::<SubmitData>().next().unwrap()?;
	assert_eq!(submit_call.value.data.0.as_slice(), DATA);
	*/

	// Note: Ideal way to get the rows for specific appData, we should use the app_specific_rows from kate recovery, which is out scope for this example
	// 1. Check query rows.
	let row_indexes = Rows::truncate_from(vec![0]);
	let mut params = RpcParams::new();
	params.push(row_indexes)?;
	params.push(block_hash)?;
	let query_rows: Vec<GRow> = client.rpc().request("kate_queryRows", params).await?;
	trace!("Query rows RPC: {query_rows:?}");

	// 3. Check proof.
	let mut rng = thread_rng();
	let cells = (0..7)
		.map(|_| {
			let col = rng.gen_range(0..256);
			Cell::new(0, col)
		})
		.collect::<Vec<_>>();
	let mut params = RpcParams::new();
	params.push(cells)?;
	params.push(block_hash)?;
	let proof: Vec<GDataProof> = client.rpc().request("kate_queryProof", params).await?;
	trace!("Query proof RPC: {proof:?}");

	Ok(())
}
