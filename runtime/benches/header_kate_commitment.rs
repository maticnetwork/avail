use avail_core::{AppExtrinsic, BlockLengthColumns, BLOCK_CHUNK_SIZE, DA_DISPATCH_RATIO_PCT};
use da_control::Config as DAConfig;
use da_runtime::Runtime;
use frame_support::traits::Get as _;
use frame_system::{limits::BlockLength, native::hosted_header_builder::hosted_header_builder};
use sp_core::H256;
use sp_std::iter::repeat;

#[allow(dead_code)]
fn make_txs(cols: BlockLengthColumns) -> Vec<AppExtrinsic> {
	let data_length: u32 = <Runtime as DAConfig>::MaxAppDataLength::get();
	let rows = <Runtime as DAConfig>::MaxBlockRows::get().0;

	let mut nb_tx = 4; // Value set depending on MaxAppDataLength (512 kb) to reach 2 mb
	let max_tx: u32 =
		rows * cols.0 * (BLOCK_CHUNK_SIZE.get().checked_sub(2).unwrap()) / data_length;
	if nb_tx > max_tx {
		nb_tx = max_tx;
	}

	let data: Vec<u8> = repeat(b'X')
		.take(usize::try_from(data_length).unwrap())
		.collect::<Vec<_>>();
	vec![AppExtrinsic::from(data); nb_tx as usize]
}

#[allow(dead_code)]
fn block_length(cols: BlockLengthColumns) -> BlockLength {
	let rows = <Runtime as DAConfig>::MaxBlockRows::get();
	BlockLength::with_normal_ratio(
		rows,
		cols,
		BLOCK_CHUNK_SIZE,
		Perbill::from_percent(DA_DISPATCH_RATIO_PCT as u32),
	)
	.unwrap()
}

#[allow(dead_code)]
fn commitment_builder_with(txs: Vec<AppExtrinsic>, block_length: BlockLength) {
	let seed = [0u8; 32];
	let root = H256::zero();
	let block_number: u32 = 0;

	let _ = hosted_header_builder::build(txs, root, block_length, block_number, seed);
}
