use avail_core::{asdr::AppUncheckedExtrinsic, block::header::Header as DaHeader, OpaqueExtrinsic};
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature,
};
use sp_std::vec::Vec;

use crate::{AllPalletsWithSystem, Runtime, RuntimeCall};

/// An index to a block.
pub type BlockNumber = u32;
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;
/// Index of a transaction in the chain.
pub type Index = u32;
/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;
/// Digest item type.
pub type DigestItem = generic::DigestItem;
/// Time type
pub type Moment = u64;

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = DaHeader;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = AppUncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// DA Block type as expected by this runtime.
pub type Block = avail_core::block::DaBlock<UncheckedExtrinsic>;
/// Block type for the node
pub type NodeBlock = generic::Block<Header, OpaqueExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// VRF Seed type.
pub type Seed = [u8; 32];

/// The SignedExtension to the basic transaction logic.
///
/// When you change this, you **MUST** modify [`sign`] in `bin/node/testing/src/keyring.rs`!
///
/// [`sign`]: <../../testing/src/keyring.rs.html>
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	da_control::CheckAppId<Runtime>,
);

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

/// ID type for named reserves.
pub type ReserveIdentifier = [u8; 8];

impl_opaque_keys! {
	pub struct SessionKeys {
		pub babe: crate::Babe,
		pub grandpa: crate::Grandpa,
		pub im_online: crate::ImOnline,
		pub authority_discovery: crate::AuthorityDiscovery,
	}
}
