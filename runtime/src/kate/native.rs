use super::{Error, GDataProof, GMultiProof, GProof, GRawScalar, GRow};
use avail_core::{AppExtrinsic, AppId, BlockLengthColumns, BlockLengthRows};
use core::num::NonZeroU16;
use frame_system::{limits::BlockLength, native::hosted_header_builder::MIN_WIDTH};
#[cfg(feature = "std")]
use kate::{
	com::Cell,
	couscous::multiproof_params,
	gridgen::{AsBytes as _, EvaluationGrid as EGrid},
	M1NoPrecomp,
};
use kate::{gridgen::ArkScalar, Seed};
use kate_recovery::matrix::Dimensions;
use sp_runtime::SaturatedConversion as _;
use sp_runtime_interface::runtime_interface;
use sp_std::vec::Vec;

#[cfg(feature = "std")]
static SRS: std::sync::OnceLock<M1NoPrecomp> = std::sync::OnceLock::new();

#[cfg(feature = "std")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};

/// Hosted function to build the header using `kate` commitments.
#[runtime_interface]
pub trait HostedKate {
	fn grid(
		submitted: Vec<AppExtrinsic>,
		block_length: BlockLength,
		seed: Seed,
		selected_rows: Vec<u32>,
	) -> Result<Vec<GRow>, Error> {
		let (max_width, max_height) = to_width_height(&block_length);
		let selected_rows = selected_rows
			.into_par_iter()
			.map(usize::try_from)
			.collect::<Result<Vec<_>, _>>()?;

		let grid = EGrid::from_extrinsics(submitted, MIN_WIDTH, max_width, max_height, seed)?
			.extend_columns(NonZeroU16::new(2).expect("2>0"))
			.map_err(|_| Error::ColumnExtension)?;
		let rows = selected_rows
			.into_par_iter()
			.map(|row_idx| {
				let row = grid.row(row_idx).ok_or(Error::MissingRow(row_idx as u32))?;
				row.iter()
					.map(|scalar| scalar.to_bytes().map(GRawScalar::from))
					.collect::<Result<Vec<_>, _>>()
					.map_err(|_| Error::InvalidScalarAtRow(row_idx as u32))
			})
			.collect::<Result<Vec<_>, _>>()?;

		Ok(rows)
	}

	fn proof(
		extrinsics: Vec<AppExtrinsic>,
		block_len: BlockLength,
		seed: Seed,
		cells: Vec<(u32, u32)>,
	) -> Result<Vec<GDataProof>, Error> {
		let srs = SRS.get_or_init(multiproof_params);
		let (max_width, max_height) = to_width_height(&block_len);
		let grid = EGrid::from_extrinsics(extrinsics, MIN_WIDTH, max_width, max_height, seed)?
			.extend_columns(NonZeroU16::new(2).expect("2>0"))
			.map_err(|_| Error::ColumnExtension)?;

		let poly = grid.make_polynomial_grid()?;

		let proofs = cells
			.into_par_iter()
			.map(|(row, col)| -> Result<GDataProof, Error> {
				let data: GRawScalar = grid
					.get(row as usize, col as usize)
					.ok_or(Error::MissingCell { row, col })?
					.to_bytes()
					.map(GRawScalar::from)
					.map_err(|_| Error::InvalidScalarAtRow(row))?;

				let cell = Cell::new(BlockLengthRows(row), BlockLengthColumns(col));
				let proof = poly
					.proof(srs, &cell)?
					.to_bytes()
					.map(GProof)
					.map_err(|_| Error::Proof)?;

				Ok((data, proof))
			})
			.collect::<Result<Vec<_>, _>>()?;

		Ok(proofs)
	}

	fn multiproof(
		extrinsics: Vec<AppExtrinsic>,
		block_len: BlockLength,
		seed: Seed,
		cells: Vec<(u32, u32)>,
	) -> Result<Vec<GMultiProof>, Error> {
		let srs = SRS.get_or_init(multiproof_params);
		let (max_width, max_height) = to_width_height(&block_len);
		let grid = EGrid::from_extrinsics(extrinsics, MIN_WIDTH, max_width, max_height, seed)?
			.extend_columns(NonZeroU16::new(2).expect("2>0"))
			.map_err(|_| Error::ColumnExtension)?;

		let poly = grid.make_polynomial_grid()?;

		let proofs = cells
			.into_par_iter()
			.map(|(row, col)| -> Result<GMultiProof, Error> {
				let cell = Cell::new(BlockLengthRows(row), BlockLengthColumns(col));
				let target_dims = Dimensions::new(16, 64).expect("16,64>0");
				// TODO: This isn't correct, need to put in the correct mp grid dim
				// TODO: safety
				if cell.row.0 >= grid.dims().height() as u32
					|| cell.col.0 >= grid.dims().width() as u32
				{
					return Err(Error::MissingCell { row, col });
				}
				let mp = poly.multiproof(srs, &cell, &grid, target_dims)?;
				let data = mp
					.evals
					.into_iter()
					.flatten()
					.map(|e: ArkScalar| {
						e.to_bytes()
							.map(GRawScalar::from)
							.map_err(|_| Error::InvalidScalarAtRow(row))
					})
					.collect::<Result<Vec<GRawScalar>, _>>()?;

				let proof = mp.proof.to_bytes().map(GProof).map_err(|_| Error::Proof)?;

				// TODO: should we also return the block coords in mp.block?
				Ok((data, proof))
			})
			.collect::<Result<Vec<GMultiProof>, _>>()?;

		Ok(proofs)
	}

	fn app_data(
		submitted: Vec<AppExtrinsic>,
		block_length: BlockLength,
		seed: Seed,
		app_id: u32,
	) -> Result<Vec<Option<GRow>>, Error> {
		let (max_width, max_height) = to_width_height(&block_length);
		let grid = EGrid::from_extrinsics(submitted, MIN_WIDTH, max_width, max_height, seed)?;

		// let orig_dims = non_extended_dims(grid.dims()).ok_or(Error::InvalidDimension)?;
		let dims = grid.dims();
		let Some(rows) = grid.app_rows(AppId(app_id), Some(dims))? else {
			return Err(Error::AppRow);
		};

		let mut all_rows = vec![None; dims.height()];
		for (row_y, row) in rows {
			let g_row = row
				.into_par_iter()
				.map(|s| s.to_bytes().map(GRawScalar::from))
				.collect::<Result<Vec<_>, _>>()
				.map_err(|_| Error::InvalidScalarAtRow(row_y as u32))?;
			all_rows[row_y] = Some(g_row);
		}

		Ok(all_rows)
	}
}

fn to_width_height(block_len: &BlockLength) -> (usize, usize) {
	// even if we run on a u16 target this is fine
	let width = block_len.cols.0.saturated_into();
	let height = block_len.rows.0.saturated_into();
	(width, height)
}
