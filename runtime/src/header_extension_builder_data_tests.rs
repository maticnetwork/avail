use super::*;
use crate::{Runtime, SignedExtra, UncheckedExtrinsic};

use avail_base::HeaderExtensionBuilderData;
use avail_core::data_proof::{BoundedData, Message, TxDataRoots};
use da_control::{AppDataFor, Call as DaCall, CheckAppId};
use frame_system::{
	CheckEra, CheckGenesis, CheckNonZeroSender, CheckNonce, CheckSpecVersion, CheckTxVersion,
	CheckWeight,
};
use pallet_balances::Call as BalancesCall;
use pallet_vector::Call as VectorCall;

use avail_core::data_proof::AddressedMessage;
use avail_core::data_proof::SubTrie;
use binary_merkle_tree::{verify_proof, Leaf, MerkleProof};
use codec::{Compact, Encode};
use derive_more::Constructor;
use hex_literal::hex;
use pallet_transaction_payment::ChargeTransactionPayment;
use sp_core::H256;
use sp_keyring::AccountKeyring::{Alice, Bob};
use sp_runtime::traits::Keccak256;
use sp_runtime::{
	generic::Era,
	traits::{SignedExtension, Verify as _},
	MultiSignature,
};
use test_case::test_case;

#[derive(Constructor)]
pub struct CallsProof {
	pub proof: MerkleProof<H256, Vec<u8>>,
	pub root: H256,
	#[allow(dead_code)]
	pub message: Option<AddressedMessage>,
}

/// Creates the Merkle Proof of the submitted data items in `calls` filtered by `F` and
/// the given `data_index`.
///
/// If `transaction_index` is greater than the number transactions in the block, it will return `None`.
/// If `data_index` is greater than the number of Merkle leaves, it will return `None`.
///
/// # TODO
/// - The `merkle_proof` requires `ExactSizeIterator`, forcing to load all submitted data into
/// memory. That would increase the memory footprint of the node significantly. We could fix this
/// adding the number of submitted data items at `System` pallet.
pub fn calls_proof(
	block: u32,
	extrinsics: &[Vec<u8>],
	leaf_idx: usize,
	call_type: SubTrie,
) -> Option<CallsProof> {
	let tx_data = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(block, &extrinsics);
	let message = tx_data
		.bridge_messages
		.get(leaf_idx)
		.map(|bridged| bridged.addr_msg.clone());

	let (proof, root) = match call_type {
		SubTrie::DataSubmit => {
			let proof = tx_data.submitted_proof_of(leaf_idx)?;
			let root = tx_data.bridged_root();
			(proof, root)
		},
		SubTrie::Bridge => {
			let proof = tx_data.bridged_proof_of(leaf_idx)?;
			let root = tx_data.submitted_root();
			(proof, root)
		},
	};

	Some(CallsProof::new(proof, root, message))
}

/// Verify Merkle Proof correctness versus given root hash.
///
/// The proof is NOT expected to contain leaf hash as the first
/// element, but only all adjacent nodes required to eventually by process of
/// concatenating and hashing end up with given root hash.
///
/// The proof must not contain the root hash.
#[allow(dead_code)]
pub fn verify<I>(
	root: H256,
	proof: I,
	number_of_submitted_data: u32,
	data_index: u32,
	data_hash: H256,
) -> bool
where
	I: IntoIterator<Item = H256>,
{
	let leaf = Leaf::Hash(data_hash);
	verify_proof::<Keccak256, _, _>(
		&root,
		proof,
		number_of_submitted_data as usize,
		data_index as usize,
		leaf,
	)
}

fn extra() -> SignedExtra {
	(
		CheckNonZeroSender::<Runtime>::new(),
		CheckSpecVersion::<Runtime>::new(),
		CheckTxVersion::<Runtime>::new(),
		CheckGenesis::<Runtime>::new(),
		CheckEra::<Runtime>::from(Era::Mortal(32, 2)),
		CheckNonce::<Runtime>::from(0),
		CheckWeight::<Runtime>::new(),
		ChargeTransactionPayment::<Runtime>::from(0),
		CheckAppId::<Runtime>::from(AppId(1)),
	)
}
fn additional_signed() -> <SignedExtra as SignedExtension>::AdditionalSigned {
	let spec_ver = VERSION.spec_version;
	let tx_ver = VERSION.transaction_version;
	let genesis = H256::default();
	let era = H256::repeat_byte(1);

	((), spec_ver, tx_ver, genesis, era, (), (), (), ())
}

fn signed_extrinsic(function: RuntimeCall) -> Vec<u8> {
	let extra = extra();
	let additional = additional_signed();
	let alice = Alice.to_account_id();

	let payload = SignedPayload::from_raw(function.clone(), extra.clone(), additional).encode();
	let signature: MultiSignature = Alice.sign(&payload).into();

	assert!(signature.verify(&*payload, &alice));
	UncheckedExtrinsic::new_signed(function, alice.into(), signature, extra).encode()
}

fn submit_data(data: Vec<u8>) -> Vec<u8> {
	let data = AppDataFor::<Runtime>::truncate_from(data);
	let function = DaCall::submit_data { data }.into();

	signed_extrinsic(function)
}

fn transfer_keep_alive() -> Vec<u8> {
	let bob = Bob.to_account_id();
	let amount = 1 * AVAIL;
	let function = BalancesCall::transfer_keep_alive {
		dest: bob.into(),
		value: amount,
	}
	.into();

	signed_extrinsic(function)
}

fn bridge_msg(data: Vec<u8>) -> Vec<u8> {
	let message = Message::ArbitraryMessage(BoundedData::truncate_from(data));
	let to = H256::repeat_byte(0x01);
	let function = VectorCall::send_message {
		message,
		to,
		domain: 2,
	}
	.into();

	signed_extrinsic(function)
}

fn bridge_fungible_msg(asset_id: H256, amount: u128) -> Vec<u8> {
	let message = Message::FungibleToken { asset_id, amount };
	let to = H256::repeat_byte(0x01);
	let function = VectorCall::send_message {
		message,
		to,
		domain: 2,
	}
	.into();

	signed_extrinsic(function)
}

fn bridge_failed_send_message_txs(failed_txs: Vec<u32>) -> Vec<u8> {
	let failed_txs: Vec<Compact<u32>> = failed_txs.into_iter().map(|i| Compact::from(i)).collect();
	let function = VectorCall::failed_send_message_txs { failed_txs }.into();
	UncheckedExtrinsic::new_unsigned(function).encode()
}

fn empty_root() -> H256 {
	let root = TxDataRoots::new(H256::zero(), H256::zero()).data_root;
	let exp_root = hex!("ad3228b676f7d3cd4284a5443f17f1962b36e491b30a40b2405849e597ba5fb5");
	assert_eq!(root.0, exp_root);
	root
}

// Data root tests
#[test_case(&[submit_data(hex!("abcd").to_vec())] => H256(hex!("f1f399f7e0d8c8ed712df0c21b4ec78f3b8533f1c3d0215e4023e1b7c80bfd91")); "submitted")]
#[test_case(&[submit_data(vec![])] => empty_root(); "empty submitted")]
#[test_case(&[] => empty_root(); "empty submitted 2")]
#[test_case(&[bridge_msg(hex!("47").to_vec())] => H256(hex!("df93f65f9f5adf3ac0d46e5a08432b96ef362bf229e1737939051884c5506e02")); "bridged data")]
#[test_case(&[bridge_fungible_msg(H256::repeat_byte(1), 1_000_000)] => H256(hex!("e93394eeaedb2158a154a29b9333fe06451fbe82c9cff5b961a6d701782450bc")) ; "bridged fungible")]
#[test_case(&[submit_data(hex!("abcd").to_vec()), bridge_msg(hex!("47").to_vec())] => H256(hex!("c925bfccfc86f15523c5b40b2bd6d8a66fc51f3d41176d77be7928cb9e3831a7")); "submitted and bridged")]
fn data_root_filter(extrinsics: &[Vec<u8>]) -> H256 {
	HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, extrinsics).data_root()
}

#[cfg(test)]
mod to_app_extrinsics_tests {
	use super::*;

	// Empty extrinsics should give empty.
	#[test]
	fn to_app_extrinsics_filters_correctly_1() {
		let extrinsics: Vec<Vec<u8>> = vec![];

		let data = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, &extrinsics)
			.to_app_extrinsics()
			.into_iter()
			.map(|app_extrinsic| app_extrinsic.data)
			.collect::<Vec<Vec<u8>>>();

		assert_eq!(extrinsics, data);
	}

	// Transfer extrinsics should give empty.
	#[test]
	fn to_app_extrinsics_filters_correctly_2() {
		let extrinsics: Vec<Vec<u8>> = vec![
			transfer_keep_alive().to_vec(),
			transfer_keep_alive().to_vec(),
		];

		let data = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, &extrinsics)
			.to_app_extrinsics()
			.into_iter()
			.map(|app_extrinsic| app_extrinsic.data)
			.collect::<Vec<Vec<u8>>>();

		let expected_data: Vec<Vec<u8>> = vec![];

		assert_eq!(expected_data, data);
	}

	// Submit data extrinsics should give submit data.
	#[test]
	fn to_app_extrinsics_filters_correctly_3() {
		let extrinsics = vec![
			submit_data(hex!("abcd").to_vec()),
			submit_data(hex!("abcd").to_vec()),
		];

		let data = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, &extrinsics)
			.to_app_extrinsics()
			.into_iter()
			.map(|app_extrinsic| app_extrinsic.data)
			.collect::<Vec<Vec<u8>>>();

		assert_eq!(extrinsics, data);
	}

	// Submit data  and transfer extrinsics should give only submit data.
	#[test]
	fn to_app_extrinsics_filters_correctly_4() {
		let extrinsics = vec![
			submit_data(hex!("abcd").to_vec()),
			transfer_keep_alive().to_vec(),
		];

		let data = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, &extrinsics)
			.to_app_extrinsics()
			.into_iter()
			.map(|app_extrinsic| app_extrinsic.data)
			.collect::<Vec<Vec<u8>>>();

		assert_eq!(vec![extrinsics[0].clone()], data);
	}

	// Submit data, empty submit data and bridge extrinsics should give only non-empty submit data.
	#[test]
	fn to_app_extrinsics_filters_correctly_5() {
		let extrinsics = vec![
			submit_data(hex!("abcd").to_vec()),
			submit_data(hex!("").to_vec()),
			bridge_msg(b"123".to_vec()),
		];

		let data = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, &extrinsics)
			.to_app_extrinsics()
			.into_iter()
			.map(|app_extrinsic| app_extrinsic.data)
			.collect::<Vec<Vec<u8>>>();

		assert_eq!(vec![extrinsics[0].clone()], data);
	}
}

#[cfg(test)]
mod bridge_tests {

	use super::*;
	use avail_base::header_extension::BridgedData;
	use sp_core::keccak_256;

	fn expected_send_arbitrary_data() -> HeaderExtensionBuilderData {
		let message = Message::ArbitraryMessage(BoundedData::truncate_from(b"123".to_vec()));
		let addr_msg = AddressedMessage::new(
			message,
			hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").into(),
			hex!("0101010101010101010101010101010101010101010101010101010101010101").into(),
			1,
			2,
			0,
		);

		let message_fungible = Message::FungibleToken {
			asset_id: H256::zero(),
			amount: 42_000_000_000_000_000_000u128,
		};
		let addr_msg_fungible = AddressedMessage::new(
			message_fungible,
			hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").into(),
			hex!("0101010101010101010101010101010101010101010101010101010101010101").into(),
			1,
			2,
			1,
		);
		HeaderExtensionBuilderData {
			bridge_messages: vec![
				BridgedData::new(0, addr_msg),
				BridgedData::new(1, addr_msg_fungible),
			],
			..Default::default()
		}
	}

	// Bridge arbitrary message, bridge fungible message and submit data tx extrinsics should produce 2 bridge messages.
	#[test]
	fn bridge_message_filter_correctly() {
		let extrinsics = vec![
			bridge_msg(b"123".to_vec()),
			bridge_fungible_msg(H256::zero(), 42_000_000_000_000_000_000u128),
			submit_data(hex!("abcd").to_vec()),
		];
		let data = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, &extrinsics);
		let expected = expected_send_arbitrary_data();
		assert_eq!(data.bridge_messages, expected.bridge_messages);
		assert_eq!(data.roots().bridge_root, expected.roots().bridge_root);
	}

	// Submit data tx extrinsics should produce no bridge message.
	#[test]
	fn bridge_message_is_empty() {
		let extrinsics: Vec<Vec<u8>> = vec![submit_data(hex!("abcd").to_vec())];
		let data = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, &extrinsics);
		let expected = HeaderExtensionBuilderData::default();
		assert_eq!(data.bridge_messages, expected.bridge_messages);
		assert_eq!(data.roots().bridge_root, expected.roots().bridge_root);
	}

	// Bridges message that are failed (and hence in bridge_failed_send_message_txs) should not generate bridge messages
	#[test]
	fn bridge_message_filter_failed_tx_correctly() {
		let extrinsics = vec![
			bridge_msg(b"123".to_vec()),
			bridge_fungible_msg(H256::zero(), 42_000_000_000_000_000_000u128),
			submit_data(hex!("abcd").to_vec()),
			bridge_failed_send_message_txs(vec![0, 1]),
		];
		let data = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, &extrinsics);
		let expected: HeaderExtensionBuilderData = HeaderExtensionBuilderData::default();
		assert_eq!(data.bridge_messages, expected.bridge_messages);
		assert_eq!(data.roots().bridge_root, expected.roots().bridge_root);
	}

	// Bridges message and roots should be same for same data and ignore failed txs
	#[test]
	fn bridge_roots_should_be_same() {
		let extrinsics_1 = vec![
			bridge_msg(b"123".to_vec()),
			bridge_fungible_msg(H256::zero(), 42_000_000_000_000_000_000u128),
			bridge_fungible_msg(H256::zero(), 42_000_000_000_000_000_000u128),
			submit_data(hex!("abcd").to_vec()),
			bridge_failed_send_message_txs(vec![1, 2]),
		];
		let data_1 = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, &extrinsics_1);

		let extrinsics_2 = vec![
			bridge_msg(b"123".to_vec()),
			submit_data(hex!("abcd").to_vec()),
			bridge_failed_send_message_txs(vec![]),
		];
		let data_2 = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, &extrinsics_2);

		assert_eq!(data_1.bridge_messages, data_2.bridge_messages);
		assert_eq!(data_1.roots(), data_2.roots());
	}

	// Check the balanced tree, 3 tx should make 4 leaves
	#[test]
	fn check_bridge_balanced_tree() {
		let extrinsics = vec![
			bridge_msg(b"123".to_vec()),
			bridge_msg(b"123".to_vec()),
			bridge_msg(b"123".to_vec()),
		];
		let data = HeaderExtensionBuilderData::from_raw_extrinsics::<Runtime>(0, &extrinsics);
		let merkle_proof = data.bridged_proof_of(2).unwrap();
		let proof = merkle_proof.proof;
		let expected_first_item_in_proof = H256(keccak_256(H256::zero().as_bytes()));

		assert_eq!(proof[0], expected_first_item_in_proof);
		assert_eq!(proof.len(), 2);
		assert_eq!(merkle_proof.number_of_leaves, 4);
	}
}

mod data_root {
	use frame_support::traits::DefensiveTruncateFrom;
	use sp_core::keccak_256;
	use sp_runtime::BoundedVec;

	use super::*;

	fn get_calls_proof(
		extrinsics: &[Vec<u8>],
		leaf_idx: usize,
	) -> Option<(MerkleProof<H256, Vec<u8>>, H256)> {
		let calls_proof = calls_proof(0, &extrinsics, leaf_idx, SubTrie::DataSubmit).unwrap();
		let proof = calls_proof.proof;
		let root = calls_proof.root;

		Some((proof, root))
	}

	#[test]
	fn test_left_data_proof_with_one_tx() {
		let exp_da_root = hex!("40105d5bc10105c17fd72b93a8f73369e2ee6eee4d4714b7bf7bf3c2f156e601");
		// leaf 0 keccak256(044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116d)
		//                  40105d5bc10105c17fd72b93a8f73369e2ee6eee4d4714b7bf7bf3c2f156e601
		assert_eq!(
			keccak_256("0".as_bytes()),
			hex!("044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116d")
		);
		assert_eq!(
			keccak_256(&keccak_256("0".as_bytes())),
			hex!("40105d5bc10105c17fd72b93a8f73369e2ee6eee4d4714b7bf7bf3c2f156e601")
		);

		let extrinsics: Vec<Vec<u8>> = vec![submit_data("0".into())];

		let (da_proof, root) = get_calls_proof(&extrinsics, 0)
			.expect("Proof not generated for the transaction index 0!");

		assert_eq!(root, H256::zero());
		assert_eq!(da_proof.leaf_index, 0);
		assert_eq!(H256(exp_da_root), da_proof.root);
		assert_eq!(da_proof.proof.len(), 0);
		assert_eq!(da_proof.number_of_leaves, 1);
	}

	#[test]
	fn test_left_data_proof_with_two_tx() {
		let exp_proof_root =
			hex!("db0ccc7a2d6559682303cc9322d4b79a7ad619f0c87d5f94723a33015550a64e");
		let exp_proof_0 = hex!("4aeff0db81e3146828378be230d377356e57b6d599286b4b517dbf8941b3e1b2");
		let extrinsics: Vec<Vec<u8>> = vec![submit_data("0".into()), submit_data("1".into())];
		// leaf 0 keccak256(044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116d)
		//                  40105d5bc10105c17fd72b93a8f73369e2ee6eee4d4714b7bf7bf3c2f156e601
		// leaf 1 keccak256(c89efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bc6)
		//                  4aeff0db81e3146828378be230d377356e57b6d599286b4b517dbf8941b3e1b2

		let (da_proof, root) = get_calls_proof(&extrinsics, 0)
			.expect("Proof not generated for the transaction index 0!");

		assert_eq!(root, H256::zero());
		assert_eq!(da_proof.leaf_index, 0);
		assert_eq!(H256(exp_proof_root), da_proof.root);
		assert_eq!(da_proof.proof.len(), 1);
		assert_eq!(da_proof.number_of_leaves, 2);
		assert_eq!(H256(exp_proof_0), da_proof.proof[0]);
	}

	#[test]
	fn test_left_data_proof_with_skipped_tx() {
		let extrinsics: Vec<Vec<u8>> = vec![
			submit_data("0".into()),
			submit_data("".into()),
			submit_data("1".into()),
			submit_data("2".into()),
		];

		// leaf 0 keccak256(044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116d)
		//                  40105d5bc10105c17fd72b93a8f73369e2ee6eee4d4714b7bf7bf3c2f156e601
		// leaf 1 keccak256(c89efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bc6)
		//                  4aeff0db81e3146828378be230d377356e57b6d599286b4b517dbf8941b3e1b2
		// leaf 2 keccak256(ad7c5bef027816a800da1736444fb58a807ef4c9603b7848673f7e3a68eb14a5)
		//                  1204b3dcd975ba0a68eafbf4d2ca0d13cc7b5e3709749c1dc36e6e74934270b3
		//  leaf appended in in order to have 2^n number of leaves
		// leaf 3 (appended) keccak256(0000000000000000000000000000000000000000000000000000000000000000)
		//                  290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e563

		// intermediate root (leaf[0], leaf[1]) db0ccc7a2d6559682303cc9322d4b79a7ad619f0c87d5f94723a33015550a64e
		// intermediate root (leaf[2], leaf[3]) 3c86bde3a90d18efbcf23e27e9b6714012aa055263fe903a72333aa9caa37f1b
		// data_root keccak256(db0ccc7a2d6559682303cc9322d4b79a7ad619f0c87d5f94723a33015550a64e, 3c86bde3a90d18efbcf23e27e9b6714012aa055263fe903a72333aa9caa37f1b)
		//                                                       (877f9ed6aa67f160e9b9b7794bb851998d15b65d11bab3efc6ff444339a3d750)

		let (da_proof, root) = get_calls_proof(&extrinsics, 0)
			.expect("Proof not generated for the transaction index 0!");

		let exp_proof_root =
			hex!("877f9ed6aa67f160e9b9b7794bb851998d15b65d11bab3efc6ff444339a3d750");
		let exp_proofs_0: [H256; 2] = [
			hex!("4aeff0db81e3146828378be230d377356e57b6d599286b4b517dbf8941b3e1b2").into(),
			hex!("3c86bde3a90d18efbcf23e27e9b6714012aa055263fe903a72333aa9caa37f1b").into(),
		];
		let exp_leaf_0 = hex!("044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116d");
		assert_eq!(root, H256::zero());
		assert_eq!(da_proof.leaf_index, 0);
		assert_eq!(H256(exp_proof_root), da_proof.root);
		assert_eq!(da_proof.proof.len(), 2);
		assert_eq!(exp_proofs_0, da_proof.proof.as_slice());
		assert_eq!(exp_leaf_0, da_proof.leaf.as_slice());
		assert_eq!(da_proof.number_of_leaves, 4);

		let (da_proof, root) = get_calls_proof(&extrinsics, 1)
			.expect("Proof not generated for the transaction index 0!");
		let exp_proof_2: [H256; 2] = [
			hex!("40105d5bc10105c17fd72b93a8f73369e2ee6eee4d4714b7bf7bf3c2f156e601").into(),
			hex!("3c86bde3a90d18efbcf23e27e9b6714012aa055263fe903a72333aa9caa37f1b").into(),
		];
		assert_eq!(root, H256::zero());
		assert_eq!(da_proof.leaf_index, 1);
		assert_eq!(H256(exp_proof_root), da_proof.root);
		assert_eq!(da_proof.proof.len(), 2);
		assert_eq!(exp_proof_2, da_proof.proof.as_slice());
		assert_eq!(da_proof.number_of_leaves, 4);

		let (da_proof, root) = get_calls_proof(&extrinsics, 2)
			.expect("Proof not generated for the transaction index 0!");

		assert_eq!(root, H256::zero());
		assert_eq!(da_proof.leaf_index, 2);

		assert_eq!(
			da_proof.root,
			hex!("877f9ed6aa67f160e9b9b7794bb851998d15b65d11bab3efc6ff444339a3d750").into()
		);

		assert_eq!(da_proof.proof.len(), 2);
		assert_eq!(
			da_proof.proof,
			[
				hex!("290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e563").into(),
				hex!("db0ccc7a2d6559682303cc9322d4b79a7ad619f0c87d5f94723a33015550a64e").into()
			]
		);
		assert_eq!(da_proof.number_of_leaves, 4);
	}

	#[test]
	fn test_message_encoding() {
		let expected_encoded_message = hex!("00000000000000000000000000000000000000000000000000000000000000200200000000000000000000000000000000000000000000000000000000000000681257bed628425a28b469114dc21a7c30205cfd00000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000de0b6b3a7640000").to_vec();

		// Message(0x02, bytes32(bytes20(0x681257BED628425a28B469114Dc21A7c30205cFD)), bytes32(uint256(1)), 2, 1, abi.encode(bytes32(0), 1 ether), 0);
		let asset_id = H256::zero();
		let amount = 1_000_000_000_000_000_000u128;
		let message = AddressedMessage {
			message: Message::FungibleToken { asset_id, amount },
			from: hex!("681257BED628425a28B469114Dc21A7c30205cFD000000000000000000000000").into(),
			to: hex!("0000000000000000000000000000000000000000000000000000000000000001").into(),
			origin_domain: 2,
			destination_domain: 1,
			id: 0,
		};

		let abi_encoded = message.abi_encode();
		assert_eq!(expected_encoded_message, abi_encoded);
	}

	#[test]
	fn test_message_encoding_from_avail_with_hash() {
		// Message is : Message {
		// message_type: FungibleToken,
		// from: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d,
		// to: 0x0000000000000000000000000000000000000000000000000000000000000001,
		// origin_domain: 1,
		// destination_domain: 2,
		// data: BoundedVec([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 102400),
		// id: 1 }

		let asset_id = H256::zero();
		let amount = 1u128;
		let message = AddressedMessage {
			message: Message::FungibleToken { asset_id, amount },
			from: hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").into(),
			to: hex!("0000000000000000000000000000000000000000000000000000000000000001").into(),
			origin_domain: 1,
			destination_domain: 2,
			id: 1,
		};

		let encoded = message.abi_encode();
		let leaf_hash = hex!("ccd6cb2b400270449e283f0f9e4fdf1dbfeb44fa5d86468272d6834d2be7574f");
		assert_eq!(leaf_hash, keccak_256(encoded.as_slice()));
	}

	#[test]
	fn test_message_encoding_from_avail_with_hash1() {
		let asset_id = H256::zero();
		let amount = 1_000_000_000_000_000_000u128;
		let message = AddressedMessage {
			message: Message::FungibleToken { asset_id, amount },
			from: hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").into(),
			to: hex!("0000000000000000000000000000000000000000000000000000000000000001").into(),
			origin_domain: 1,
			destination_domain: 2,
			id: 1,
		};

		let encoded = message.abi_encode();
		let leaf_hash = hex!("94491650baa28a6f0db3c5e9495e12e43b7f1b2726fa5c5dabed2619514bd7b5");
		assert_eq!(leaf_hash, keccak_256(encoded.as_slice()));
	}

	#[test]
	fn test_amb_message_encoding() {
		let expected_encoding = hex!("000000000000000000000000000000000000000000000000000000000000002001000000000000000000000000000000000000000000000000000000000000008f8d47bf15953e26c622f36f3366e43e26b9b78b000000000000000000000000c437b127628aa7984af0f001dc7ac023eee266f0df6356ef9243f340af8842360000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000002b67");
		let expected_hash =
			hex!("ad79a34ee43ea39301b1f190558ea279122328ebd66b342a49a131ee5befd3b5");

		let data = hex!("0000000000000000000000000000000000000000000000000000000000002b67");
		let data = BoundedVec::defensive_truncate_from(data.to_vec());

		let message = AddressedMessage {
			message: Message::ArbitraryMessage(data),
			from: H256(hex!(
				"8f8d47bf15953e26c622f36f3366e43e26b9b78b000000000000000000000000"
			)),
			to: H256(hex!(
				"c437b127628aa7984af0f001dc7ac023eee266f0df6356ef9243f340af884236"
			)),
			origin_domain: 2,
			destination_domain: 1,
			id: 1,
		};
		let encoded_message = message.abi_encode();
		assert_eq!(&expected_encoding, encoded_message.as_slice());
		assert_eq!(expected_hash, keccak_256(encoded_message.as_slice()));
	}

	#[test]
	fn test_amb_message_encoding_with_hash_check() {
		let expected_encoding = hex!("00000000000000000000000000000000000000000000000000000000000000200100000000000000000000000000000000000000000000000000000000000000681257bed628425a28b469114dc21a7c30205cfd0000000000000000000000003547517355657647456b6f7847444a5044576251694b4478714b6d675a3570470000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000d48656c6c6f2c20576f726c642100000000000000000000000000000000000000");
		let expected_hash =
			hex!("5774ba3f9618e2da3885b0e2853e4005c3e836625e8be0f69bf3d93f51fac58d");
		let data = BoundedVec::defensive_truncate_from("Hello, World!".as_bytes().to_vec());

		let message = AddressedMessage {
			message: Message::ArbitraryMessage(data),
			from: hex!("681257BED628425a28B469114Dc21A7c30205cFD000000000000000000000000").into(),
			to: hex!("3547517355657647456b6f7847444a5044576251694b4478714b6d675a357047").into(),
			origin_domain: 2,
			destination_domain: 1,
			id: 0,
		};

		let encoded_message = message.abi_encode();
		assert_eq!(&expected_encoding, encoded_message.as_slice());
		assert_eq!(expected_hash, keccak_256(encoded_message.as_slice()));
	}
}
