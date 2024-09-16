// This file is part of Substrate.

// Copyright (C) 2021-2022 Parity Technologies (UK) Ltd.
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

//! Provide types to help defining a mock environment when testing pallets.

use avail_core::{
	traits::{GetAppId, MaybeCaller},
	AppId, OpaqueExtrinsic,
};
use codec::{Decode, Encode};
use frame_support::traits::ExtrinsicCall;
use scale_info::TypeInfo;
use sp_runtime::{
	generic,
	traits::{DispatchInfoOf, Extrinsic, ExtrinsicMetadata, SignedExtension},
	transaction_validity::TransactionValidityError,
};

use crate::{native::hosted_header_builder::da::BlockNumber, Config};

#[derive(Clone, Copy, Default, Debug, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub struct DefaultGetAppId {}

impl GetAppId for DefaultGetAppId {}
impl SignedExtension for DefaultGetAppId {
	type AccountId = u32;
	type AdditionalSigned = ();
	type Call = ();
	type Pre = ();

	const IDENTIFIER: &'static str = "DefaultGetAppId";

	fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
		Ok(())
	}

	fn pre_dispatch(
		self,
		_who: &Self::AccountId,
		_call: &Self::Call,
		_info: &DispatchInfoOf<Self::Call>,
		_len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		Ok(())
	}
}

#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
/// An unchecked extrinsic type to be used in tests.
pub struct MockUncheckedExtrinsic<T: Config>(
	pub  generic::UncheckedExtrinsic<
		<T as Config>::AccountId,
		<T as Config>::RuntimeCall,
		(),
		DefaultGetAppId,
	>,
);

impl<T: Config> Extrinsic for MockUncheckedExtrinsic<T> {
	type Call = <T as Config>::RuntimeCall;
	type SignaturePayload = (<T as Config>::AccountId, (), DefaultGetAppId);
}

impl<T: Config> ExtrinsicMetadata for MockUncheckedExtrinsic<T> {
	type SignedExtensions = DefaultGetAppId;

	const VERSION: u8 = avail_core::asdr::EXTRINSIC_FORMAT_VERSION;
}

impl<T: Config> ExtrinsicCall for MockUncheckedExtrinsic<T> {
	fn call(&self) -> &Self::Call {
		&self.0.function
	}
}

impl<T: Config> MaybeCaller<T::AccountId> for MockUncheckedExtrinsic<T> {
	fn caller(&self) -> Option<&T::AccountId> {
		self.0.signature.as_ref().map(|s| &s.0)
	}
}

impl<T: Config> GetAppId for MockUncheckedExtrinsic<T> {
	fn app_id(&self) -> AppId {
		AppId::default()
	}
}

#[cfg(feature = "std")]
impl<T: Config> serde::Serialize for MockUncheckedExtrinsic<T> {
	fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
	where
		S: ::serde::Serializer,
	{
		let encoded = self.encode();
		sp_core::bytes::serialize(&encoded, s)
	}
}

#[cfg(feature = "std")]
impl<'a, T: Config> serde::Deserialize<'a> for MockUncheckedExtrinsic<T> {
	fn deserialize<D>(de: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'a>,
	{
		let r = sp_core::bytes::deserialize(de)?;
		Decode::decode(&mut &r[..])
			.map_err(|e| serde::de::Error::custom(format!("Decode error: {}", e)))
	}
}

impl<T: Config> TryFrom<&[u8]> for MockUncheckedExtrinsic<T> {
	type Error = codec::Error;

	fn try_from(raw: &[u8]) -> Result<Self, Self::Error> {
		let encoded = raw.encode();
		let ut = Self::decode(&mut encoded.as_slice())?;

		Ok(ut)
	}
}

impl<T: Config> TryFrom<&OpaqueExtrinsic> for MockUncheckedExtrinsic<T> {
	type Error = codec::Error;

	fn try_from(opaque: &OpaqueExtrinsic) -> Result<Self, Self::Error> {
		let encoded = opaque.encode();
		Self::decode(&mut encoded.as_slice())
	}
}

/// An implementation of `sp_runtime::traits::Block` to be used in tests.
pub type MockBlock<T> = generic::Block<
	generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	MockUncheckedExtrinsic<T>,
>;

/// An implementation of `sp_runtime::traits::Block` with DA header to be used in tests
pub type MockDaBlock<T> = avail_core_block::DaBlock<MockUncheckedExtrinsic<T>>;
