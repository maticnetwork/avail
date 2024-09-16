// !!!!
// More info about how runtime interfaces work: (https://docs.rs/sp-runtime-interface/latest/sp_runtime_interface/attr.runtime_interface.html
// !!!!

use crate::{limits::BlockLength, Config, LOG_TARGET};
use avail_core_block::{header::HeaderExtension, header::HeaderVersion, ExtendedHeader};
use avail_core::{
	AppExtrinsic,
};
pub use kate::{
	metrics::{IgnoreMetrics, Metrics},
	Seed,
};

use frame_support::traits::Randomness;
use sp_core::H256;
use sp_runtime::traits::Hash;
use sp_runtime_interface::runtime_interface;
use sp_std::vec::Vec;

pub const MIN_WIDTH: usize = 4;

pub mod da {
	use core::marker::PhantomData;

	use avail_core_block::header::{Header as DaHeader, HeaderExtension};
	use sp_runtime::traits::BlakeTwo256;

	use super::*;

	pub type Hash = sp_core::H256;
	pub type BlockNumber = u32;

	/// avail-node Header builder.
	pub struct HeaderExtensionBuilder<T: Config>(PhantomData<T>);

	impl<T: Config> super::HeaderExtensionBuilder for HeaderExtensionBuilder<T> {
		type Header = DaHeader;

		#[inline]
		fn build(
			submitted: Vec<AppExtrinsic>,
			data_root: H256,
			block_length: BlockLength,
			block_number: u32,
		) -> HeaderExtension {
			let seed = Self::random_seed::<T>();

			super::hosted_header_builder::build(
				submitted,
				data_root,
				block_length,
				block_number,
				seed,
			)
		}
	}
}

/// Trait for header builder.
pub trait HeaderExtensionBuilder {
	type Header: ExtendedHeader<Extension = HeaderExtension>;

	/// Creates the header using the given parameters.
	fn build(
		app_extrinsics: Vec<AppExtrinsic>,
		data_root: H256,
		block_length: BlockLength,
		block_number: u32,
	) -> HeaderExtension;

	/// Generates a random seed using the _epoch seed_ and the _current block_ returned by
	/// `T::Randomness` type.
	fn random_seed<T: Config>() -> Seed {
		let seed = if cfg!(feature = "secure_padding_fill") {
			let (epoch_seed, block_number) = <T as Config>::Randomness::random_seed();
			let seed = <T as Config>::Hashing::hash_of(&(&epoch_seed, &block_number));
			log::trace!(
				target: LOG_TARGET,
				"Header builder seed {seed:?} from epoch seed {epoch_seed:?} and block {block_number:?}");
			seed
		} else {
			<T as Config>::Hash::default()
		};

		seed.into()
	}
}

/// Hosted function to build the header using `kate` commitments.
#[runtime_interface]
pub trait HostedHeaderBuilder {
	/// Note: Whenever a new header version is introduced, ensure to create a corresponding version
	/// of the `build` hosted function, while retaining the existing ones.
	#[version(1)]
	fn build(
		submitted: Vec<AppExtrinsic>,
		data_root: H256,
		block_length: BlockLength,
		block_number: u32,
		seed: Seed,
	) -> HeaderExtension {
		crate::native::build_extension_v1::build_extension(
			submitted,
			data_root,
			block_length,
			block_number,
			seed,
			HeaderVersion::V3,
		)
	}

	/// Note: Whenever a new header version is introduced, ensure to create a corresponding version
	/// of the `build` hosted function, while retaining the existing ones.
	#[version(2)]
	fn build(
		submitted: Vec<AppExtrinsic>,
		data_root: H256,
		block_length: BlockLength,
		block_number: u32,
		seed: Seed,
	) -> HeaderExtension {
		crate::native::build_extension_v2::build_extension(
			submitted,
			data_root,
			block_length,
			block_number,
			seed,
			HeaderVersion::V3,
		)
	}
}
