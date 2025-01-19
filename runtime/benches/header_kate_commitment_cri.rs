use core::time::Duration;

#[cfg(feature = "codspeed")]
use codspeed_criterion_compat::{
	criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
#[cfg(not(feature = "codspeed"))]
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

include!("header_kate_commitment.rs");

fn commitment_builder(c: &mut Criterion) {
	let mut group = c.benchmark_group("commitment_builder");
	for columns in [64, 128, 256].iter() {
		let block_columns = BlockLengthColumns(*columns);
		let block_length = block_length(block_columns);
		let txs = make_txs(block_columns);

		group.throughput(Throughput::Elements(*columns as u64));
		group.bench_with_input(
			BenchmarkId::from_parameter(format!("{block_columns:?}")),
			&txs,
			|b, txs| {
				b.iter_batched(
					|| (txs.clone(), block_length.clone()),
					|(txs, block_len)| commitment_builder_with(txs, block_len),
					criterion::BatchSize::SmallInput,
				);
			},
		);
	}
	group.finish();
}

criterion_group!(
	name = benches;
	config = Criterion::default().sample_size(10).measurement_time( Duration::from_secs(60) );
	targets = commitment_builder);
criterion_main!(benches);
