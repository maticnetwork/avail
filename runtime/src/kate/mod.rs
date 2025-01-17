pub mod native;
pub mod runtime;

// Reexport
pub use runtime::{grid, multiproof, proof};

use codec::{Decode, Encode};
use core::num::TryFromIntError;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::U256;
use sp_runtime_interface::pass_by::{PassByCodec, PassByInner};
use sp_std::vec::Vec;
use thiserror_no_std::Error;

#[cfg(feature = "std")]
use kate::{com::Error as KateError, gridgen::AppRowError as KateAppRowError};

pub type GRawScalar = U256;
pub type GRow = Vec<GRawScalar>;
pub type GDataProof = (GRawScalar, GProof);
pub type GMultiProof = (Vec<GRawScalar>, GProof);

/// # NOTE
/// `Serde` requires a custom implementation for `GProof` due to the array size (greater than `[T;32]`).
/// In this case, we transform into a `Vec<u8>` as intermediate step to serialize/deserialize.
#[derive(Encode, Decode, TypeInfo, PassByInner, Debug, Clone, Copy)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(try_from = "Vec<u8>", into = "Vec<u8>"))]
pub struct GProof([u8; 48]);

impl From<GProof> for Vec<u8> {
	fn from(proof: GProof) -> Self {
		proof.0.to_vec()
	}
}

impl TryFrom<Vec<u8>> for GProof {
	type Error = u32;
	fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
		if data.len() != 48 {
			return Err(data.len() as u32);
		};

		let mut proof = [0u8; 48];
		proof.copy_from_slice(&data);
		Ok(GProof(proof))
	}
}

#[derive(Error, Encode, Decode, TypeInfo, PassByCodec, Debug)]
pub enum Error {
	#[error("Invalid integer conversion")]
	TryFromInt,
	#[error("Missing row {0}")]
	MissingRow(u32),
	#[error("Invalid scalar at row {0}")]
	InvalidScalarAtRow(u32),
	#[error("Grid generation error")]
	KateGrid,
	#[error("Invalid grid dimension")]
	InvalidDimension,
	#[error("App Data row error")]
	AppRow,
	#[error("Missing cell {row} {col}")]
	MissingCell { row: u32, col: u32 },
	#[error("MultiProof error")]
	Proof,
	#[error("Failed to extend columns")]
	ColumnExtension,
}

impl From<TryFromIntError> for Error {
	fn from(_: TryFromIntError) -> Self {
		Self::TryFromInt
	}
}

#[cfg(feature = "std")]
impl From<KateError> for Error {
	fn from(_: KateError) -> Self {
		Self::KateGrid
	}
}

#[cfg(feature = "std")]
impl From<KateAppRowError> for Error {
	fn from(_: KateAppRowError) -> Self {
		Self::AppRow
	}
}
