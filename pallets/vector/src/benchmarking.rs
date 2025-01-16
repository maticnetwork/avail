use crate::{
	state::Configuration, BalanceOf, Call, Config, ConfigurationStorage, ExecutionStateRoots,
	FunctionIds, FunctionInput, FunctionOutput, FunctionProof, Headers, Pallet, ProofInput,
	PublicValuesInput, RotateVerificationKey, StepVerificationKey, Updater, ValidProof,
};
use avail_core::data_proof::BOUNDED_DATA_MAX_LENGTH;
use avail_core::data_proof::{AddressedMessage, Message};
use frame_benchmarking::{
	impl_benchmark_test_suite, v2::benchmarks, whitelisted_caller, BenchmarkError,
};
use frame_support::traits::DefensiveTruncateFrom;
use frame_support::{traits::Currency, BoundedVec};
use frame_system::RawOrigin;
use hex_literal::hex;
use sp_core::{Get, H256};
use sp_runtime::traits::Bounded;
use sp_std::vec;

const ACCOUNT1: [u8; 32] = [2u8; 32];
pub const STEP_FUNCTION_ID: H256 = H256(hex!(
	"af44af6890508b3b7f6910d4a4570a0d524769a23ce340b2c7400e140ad168ab"
));
pub const ROTATE_FUNCTION_ID: H256 = H256(hex!(
	"9c1096d800fc42454d2d76e6ae1d461b5a67c7b474efb9d47989e47ed39b1b7b"
));
pub const STEP_VK: &str = r#"{"vk_json":{
	"protocol": "groth16",
	"curve": "bn128",
	"nPublic": 2,
	"vk_alpha_1": [
		"20491192805390485299153009773594534940189261866228447918068658471970481763042",
		"9383485363053290200918347156157836566562967994039712273449902621266178545958",
		"1"
	],
	"vk_beta_2": [
		[
			"6375614351688725206403948262868962793625744043794305715222011528459656738731",
			"4252822878758300859123897981450591353533073413197771768651442665752259397132"
		],
		[
			"10505242626370262277552901082094356697409835680220590971873171140371331206856",
			"21847035105528745403288232691147584728191162732299865338377159692350059136679"
		],
		[
			"1",
			"0"
		]
	],
	"vk_gamma_2": [
		[
			"10857046999023057135944570762232829481370756359578518086990519993285655852781",
			"11559732032986387107991004021392285783925812861821192530917403151452391805634"
		],
		[
			"8495653923123431417604973247489272438418190587263600148770280649306958101930",
			"4082367875863433681332203403145435568316851327593401208105741076214120093531"
		],
		[
			"1",
			"0"
		]
	],
	"vk_delta_2": [
		[
			"677302577815076814357170457144294271294364985082280272249076505900964830740",
			"5628948730667472013190771331033856457010306836153142947462627646651446565415"
		],
		[
			"5877290568297658003612857476419103064356778304319760331670835003648166891449",
			"10874997846396459971354014654692242947705540424071616448481145872912634110727"
		],
		[
			"1",
			"0"
		]
	],
	"vk_alphabeta_12": [],
	"IC": [
		[
			"202333273032481017331373350816007583026713320195536354260471885571526195724",
			"8246242704115088390751476790768744984402990892657920674334938931948100192840",
			"1"
		],
		[
			"12901454334783146822957332552289769626984444933652541503990843020723194328882",
			"12436078488518552293095332739673622487901350475115357313978341690183990059269",
			"1"
		],
		[
			"12828056956769114977702246128118682473179646035440405756936949778100648490262",
			"7351319165217643779735289066901404053730163225836026220896225559268517203790",
			"1"
		]
	]
}}"#;
pub const ROTATE_VK: &str = r#"{"vk_json":{
    "protocol": "groth16",
    "curve": "bn128",
    "nPublic": 2,
    "vk_alpha_1": [
        "20491192805390485299153009773594534940189261866228447918068658471970481763042",
        "9383485363053290200918347156157836566562967994039712273449902621266178545958",
        "1"
    ],
    "vk_beta_2": [
        [
            "6375614351688725206403948262868962793625744043794305715222011528459656738731",
            "4252822878758300859123897981450591353533073413197771768651442665752259397132"
        ],
        [
            "10505242626370262277552901082094356697409835680220590971873171140371331206856",
            "21847035105528745403288232691147584728191162732299865338377159692350059136679"
        ],
        [
            "1",
            "0"
        ]
    ],
    "vk_gamma_2": [
        [
            "10857046999023057135944570762232829481370756359578518086990519993285655852781",
            "11559732032986387107991004021392285783925812861821192530917403151452391805634"
        ],
        [
            "8495653923123431417604973247489272438418190587263600148770280649306958101930",
            "4082367875863433681332203403145435568316851327593401208105741076214120093531"
        ],
        [
            "1",
            "0"
        ]
    ],
    "vk_delta_2": [
        [
            "2864156988502350018268114524769442611229738724281856064310359811414088775164",
            "19784911050814990253005325251017779746002278450060367709911093357779852409724"
        ],
        [
            "2320747355788118605608963241136772405889379999161258135797985959373766905799",
            "7118041328407665643077665093375077236507031390654037220453830314560753892708"
        ],
        [
            "1",
            "0"
        ]
    ],
    "vk_alphabeta_12": [],
    "IC": [
        [
            "15615341388138779177592192310982411536626378440854127969627902314302018589756",
            "15825561397777957655855081872509949298182852212017977148985160662370122761845",
            "1"
        ],
        [
            "21866659777455953012076240694890418723891531368136637553921599064988704009798",
            "18794682133425820197214508210971026410261369883290190279860606526851568182754",
            "1"
        ],
        [
            "17134706853007662603932468543386586959990776778768283640697616786730646170163",
            "20580957029031123131958004810864543174606183854578157485523871304119815226629",
            "1"
        ]
    ]
}}"#;

// Generated with SP1 Helios https://github.com/succinctlabs/sp1-helios/blob/main/README.md
// cargo prove key —-elf (sp1 helios elf) in SP1 Helios
const SP1_VERIFICATION_KEY: [u8; 32] =
	hex!("00525ee0ed579ba497061070aea9d4918808790c2aa2b5dcb8fe94bdde4ddbb6");

#[benchmarks(where
[u8; 32]: From << T as frame_system::Config >::AccountId >,
< T as frame_system::Config >::AccountId: From < [u8; 32] >,
)]
mod benchmarks {
	use super::*;
	use crate::{Broadcasters, SP1VerificationKey, SyncCommitteeHashes};
	use sp_runtime::traits::AccountIdConversion;

	#[benchmark]
	fn send_message_arbitrary_message(
		l: Linear<0, BOUNDED_DATA_MAX_LENGTH>,
	) -> Result<(), BenchmarkError> {
		let origin = RawOrigin::Signed(whitelisted_caller());
		let message = Message::ArbitraryMessage(BoundedVec::truncate_from([0, 1, 2, 3].to_vec()));
		let to = ROTATE_FUNCTION_ID;
		let domain = 2;

		#[extrinsic_call]
		send_message(origin, message, to, domain);

		Ok(())
	}

	#[benchmark]
	fn send_message_fungible_token() -> Result<(), BenchmarkError> {
		let account = T::AccountId::from(ACCOUNT1);
		let origin = RawOrigin::Signed(account.clone());
		let message = Message::FungibleToken {
			asset_id: H256::zero(),
			amount: 1_000_000_000_000_000_000u128,
		};
		let to = ROTATE_FUNCTION_ID;
		let domain = 2;

		// ACCOUNT1 needs to have enough funds to send 1 token
		T::Currency::make_free_balance_be(&account, BalanceOf::<T>::max_value() / 2u32.into());

		#[extrinsic_call]
		send_message(origin, message, to, domain);

		Ok(())
	}

	#[benchmark]
	fn set_poseidon_hash() -> Result<(), BenchmarkError> {
		#[extrinsic_call]
		_(
			RawOrigin::Root,
			0,
			BoundedVec::truncate_from(
				hex!("0ab2afdc05c8b6ae1f2ab20874fb4159e25d5c1d4faa41aee232d6ab331332df").to_vec(),
			),
		);

		Ok(())
	}

	#[benchmark]
	fn set_broadcaster() -> Result<(), BenchmarkError> {
		let broadcaster = H256(hex!(
			"4554480000000000000000000000000000000000000000000000000000000000"
		));

		#[extrinsic_call]
		_(RawOrigin::Root, 0, broadcaster);

		Ok(())
	}

	#[benchmark]
	fn set_whitelisted_domains() -> Result<(), BenchmarkError> {
		let domains = BoundedVec::try_from(vec![1, 2, 3]).unwrap();

		#[extrinsic_call]
		_(RawOrigin::Root, domains);

		Ok(())
	}

	#[benchmark]
	fn set_configuration() -> Result<(), BenchmarkError> {
		let config = Configuration {
			slots_per_period: 20,
			finality_threshold: 20,
		};

		#[extrinsic_call]
		_(RawOrigin::Root, config);

		Ok(())
	}

	#[benchmark]
	fn source_chain_froze() -> Result<(), BenchmarkError> {
		#[extrinsic_call]
		_(RawOrigin::Root, 0, true);

		Ok(())
	}

	#[benchmark]
	fn fulfill_call_step() -> Result<(), BenchmarkError> {
		let hash = BoundedVec::truncate_from(
			hex!("0ab2afdc05c8b6ae1f2ab20874fb4159e25d5c1d4faa41aee232d6ab331332df").to_vec(),
		);

		Pallet::<T>::set_poseidon_hash(RawOrigin::Root.into(), 931, hash).unwrap();

		Updater::<T>::set(H256(ACCOUNT1));
		ConfigurationStorage::<T>::set(Configuration {
			slots_per_period: 8192,
			finality_threshold: 461,
		});

		let account = T::AccountId::from(ACCOUNT1);
		let origin = RawOrigin::Signed(account.clone());

		// We use test values instead of dev / prod values
		// We override dev config
		FunctionIds::<T>::set(Some((STEP_FUNCTION_ID, ROTATE_FUNCTION_ID)));
		StepVerificationKey::<T>::set(Some(
			BoundedVec::try_from(STEP_VK.as_bytes().to_vec()).unwrap(),
		));
		RotateVerificationKey::<T>::set(Some(
			BoundedVec::try_from(ROTATE_VK.as_bytes().to_vec()).unwrap(),
		));

		#[extrinsic_call]
		fulfill_call(
			origin,
			STEP_FUNCTION_ID,
			get_valid_step_input(),
			get_valid_step_output(),
			get_valid_step_proof(),
			7634942,
		);

		Ok(())
	}

	#[benchmark]
	fn fulfill_call_rotate() -> Result<(), BenchmarkError> {
		let slot = 7634942;
		let hash = BoundedVec::truncate_from(
			hex!("0ab2afdc05c8b6ae1f2ab20874fb4159e25d5c1d4faa41aee232d6ab331332df").to_vec(),
		);

		Pallet::<T>::set_poseidon_hash(RawOrigin::Root.into(), 931, hash).unwrap();

		Updater::<T>::set(H256(ACCOUNT1));
		ConfigurationStorage::<T>::set(Configuration {
			slots_per_period: 8192,
			finality_threshold: 342,
		});

		Headers::<T>::set(
			slot,
			H256(hex!(
				"e882fe800bed07205bf2cbf17f30148b335d143a91811ff65280c221c9f57856"
			)),
		);

		let account = T::AccountId::from(ACCOUNT1);
		let origin = RawOrigin::Signed(account.clone());

		// We use test values instead of dev / prod values
		// We override dev config
		FunctionIds::<T>::set(Some((STEP_FUNCTION_ID, ROTATE_FUNCTION_ID)));
		StepVerificationKey::<T>::set(Some(
			BoundedVec::try_from(STEP_VK.as_bytes().to_vec()).unwrap(),
		));
		RotateVerificationKey::<T>::set(Some(
			BoundedVec::try_from(ROTATE_VK.as_bytes().to_vec()).unwrap(),
		));

		#[extrinsic_call]
		fulfill_call(
			origin,
			ROTATE_FUNCTION_ID,
			get_valid_rotate_input(),
			get_valid_rotate_output(),
			get_valid_rotate_proof(),
			slot,
		);

		Ok(())
	}

	#[benchmark]
	fn execute_fungible_token() -> Result<(), BenchmarkError> {
		let slot = 8581263;
		Broadcasters::<T>::set(
			2,
			H256(hex!(
				"DC3542b6fcC39dC0d51ecdCbc6Fbb130D5e48d95000000000000000000000000"
			)),
		);

		ExecutionStateRoots::<T>::set(
			slot,
			H256(hex!(
				"c42310d65b1e953e8864480367a03179d6bd78d4ca522a5a977d2801b9b2e1d9"
			)),
		);
		let account = T::AccountId::from(ACCOUNT1);
		let pot = T::AccountId::from(T::PalletId::get().into_account_truncating());
		// ACCOUNT1 needs to have enough funds to send 1 token
		T::Currency::make_free_balance_be(&account, BalanceOf::<T>::max_value() / 2u32.into());
		T::Currency::make_free_balance_be(&pot, BalanceOf::<T>::max_value() / 2u32.into());

		let account_proof = get_valid_account_proof();
		let storage_proof = get_valid_storage_proof();
		let message = get_valid_message();
		let account = T::AccountId::from(ACCOUNT1);
		let origin = RawOrigin::Signed(account.clone());

		#[extrinsic_call]
		// amount in message 1000000000000000000
		execute(origin, slot, message, account_proof, storage_proof);

		Ok(())
	}

	#[benchmark]
	fn execute_arbitrary_message(
		l: Linear<0, BOUNDED_DATA_MAX_LENGTH>,
	) -> Result<(), BenchmarkError> {
		let hash = H256(hex!(
			"Aa8c1bFC413e00884A7ac991851686D27b387997000000000000000000000000"
		));
		Broadcasters::<T>::set(2, hash);

		let slot = 5085118;

		ExecutionStateRoots::<T>::set(
			slot,
			H256(hex!(
				"c42310d65b1e953e8864480367a03179d6bd78d4ca522a5a977d2801b9b2e1d9"
			)),
		);

		let account = T::AccountId::from(ACCOUNT1);
		let origin = RawOrigin::Signed(account.clone());
		T::Currency::make_free_balance_be(&account, BalanceOf::<T>::max_value() / 2u32.into());
		T::Currency::make_free_balance_be(
			&Pallet::<T>::account_id(),
			BalanceOf::<T>::max_value() / 2u32.into(),
		);

		let account_proof = get_valid_amb_account_proof();
		let storage_proof = get_valid_amb_storage_proof();
		let message = get_valid_amb_message();
		#[extrinsic_call]
		// amount in message 1000000000000000000
		execute(origin, slot, message, account_proof, storage_proof);

		Ok(())
	}

	#[benchmark]
	fn set_function_ids() -> Result<(), BenchmarkError> {
		let origin = RawOrigin::Root;

		#[extrinsic_call]
		_(origin, Some((STEP_FUNCTION_ID, ROTATE_FUNCTION_ID)));

		Ok(())
	}

	#[benchmark]
	fn set_updater() -> Result<(), BenchmarkError> {
		let origin = RawOrigin::Root;

		#[extrinsic_call]
		_(origin, H256(ACCOUNT1));

		Ok(())
	}

	#[benchmark]
	fn set_step_verification_key() -> Result<(), BenchmarkError> {
		let origin = RawOrigin::Root;
		let value = Some(BoundedVec::try_from(STEP_VK.as_bytes().to_vec()).unwrap());

		#[extrinsic_call]
		_(origin, value);

		Ok(())
	}

	#[benchmark]
	fn set_rotate_verification_key() -> Result<(), BenchmarkError> {
		let origin = RawOrigin::Root;
		let value = Some(BoundedVec::try_from(ROTATE_VK.as_bytes().to_vec()).unwrap());

		#[extrinsic_call]
		_(origin, value);

		Ok(())
	}

	#[benchmark]
	fn set_sp1_verification_key() -> Result<(), BenchmarkError> {
		let origin = RawOrigin::Root;
		let value = H256(SP1_VERIFICATION_KEY);

		#[extrinsic_call]
		_(origin, value);

		Ok(())
	}

	#[benchmark]
	fn set_sync_committee_hash() -> Result<(), BenchmarkError> {
		let origin = RawOrigin::Root;
		let sync_committee_hash = H256(hex!(
			"f4887c7e675fa7c166c1d17e03d0dd746aa595756b66a8fb8d8fad1215d4caaf"
		));
		let period = 754;

		#[extrinsic_call]
		_(origin, period, sync_committee_hash);

		Ok(())
	}

	#[benchmark]
	fn fulfill() -> Result<(), BenchmarkError> {
		SP1VerificationKey::<T>::set(H256(SP1_VERIFICATION_KEY));

		let slots_per_period = 8192;
		let finality_threshold = 342u16;
		let slot = 6724864u64;
		let current_period = slot / slots_per_period;

		SyncCommitteeHashes::<T>::set(
			current_period,
			H256(hex!(
				"cfde56afdd0992f3fc0a576c2b8959f66a88d47f2886046c104fe6e14da386ac"
			)),
		);

		ConfigurationStorage::<T>::set(Configuration {
			slots_per_period,
			finality_threshold,
		});

		Updater::<T>::set(H256(ACCOUNT1));
		let account = T::AccountId::from(ACCOUNT1);
		let origin = RawOrigin::Signed(account.clone());

		#[extrinsic_call]
		_(origin, get_valid_sp1_proof(), get_valid_public_values());

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}

pub fn get_valid_message() -> AddressedMessage {
	let asset_id = H256::zero();
	let amount = 1_000_000_000_000_000_000u128;
	let from = hex!("681257BED628425a28B469114Dc21A7c30205cFD000000000000000000000000");
	let to = hex!("0000000000000000000000000000000000000000000000000000000000000001");

	AddressedMessage {
		message: Message::FungibleToken { asset_id, amount },
		from: from.into(),
		to: to.into(),
		origin_domain: 2,
		destination_domain: 1,
		id: 0,
	}
}

fn get_valid_amb_message() -> AddressedMessage {
	let recipient = hex!("3547517355657647456b6f7847444a5044576251694b4478714b6d675a357047");
	let from = hex!("681257BED628425a28B469114Dc21A7c30205cFD000000000000000000000000");
	let data = BoundedVec::defensive_truncate_from("Hello, World!".as_bytes().to_vec());

	AddressedMessage {
		message: Message::ArbitraryMessage(data),
		from: from.into(),
		to: recipient.into(),
		origin_domain: 2,
		destination_domain: 1,
		id: 0,
	}
}

fn get_valid_account_proof() -> ValidProof {
	BoundedVec::truncate_from(vec![
        BoundedVec::truncate_from(hex!("f90211a00b9369252d91d5240966845819a5e73a8e7b4b531b9f66bd652df2ef25999e8fa00fbedaed13519e0d8971ca3e8f54fa6250b8c60ae522924ea5b7bc54d7ab049ca0e9a41917172ecd79fbbc0db3c117a990464df2767be84bc5fd749cb50763834fa0420cf2bd2e7aac8cc33b0731d7546b738315ac482524aebc60e39a7d0359ce84a007e78eae37bbc3d25305685e8321582390d9631365f5c40612d111c7c1d2a35aa0f90094ebbf4d71dc00d9ff469a149d62529289b7a88d63f69ef73c45ffadd9b8a0bb5c4d709b179a0dec8c3e298249746c5f36f1ab4c7bb0985e4c565bfa96efd2a0b44d9462647f7f99afe5396fc6c049ca3d343748d9eb27f64e118a4d55d26d2ba08119035469c78ffe7336ba8c0117906ca64c9827f16ef71b5fcd99272d983779a067a02fa9917bda0acfb3646ba7e91c7e54e56e40342c552c0b069f949a64e122a0d2b39ca49bbaf2582e8c48243c86249ef034d14f8272952975c1f01773d508c4a0cd89136cd6f4203f52553a6856ba48ee08fe3d0165b5d020b4781ec0b0195bfda0bfdc0af814301e39e11e3c2186b3969b5c6ffdcb627d99f2e1e425d48300253ea0689a72bc1f0084b35849c938c2298e0d90384a071b6ef7bb145dab1b1205efffa0614a4f66f2451debf003b8584d9c2bbb87ee1d141cb240b494a840a91d32e126a01885c00eb4d6365b341f3c21bc44754f901ff56e957399069209b612c272a97880").to_vec()),
        BoundedVec::truncate_from(hex!("f90211a0c3c1510781a6c8ba9dae550cd691248cbfe8e3071987f0da92f13d03402e68b7a0267ee923bf851b5f9776eebb0c09be120dcb22c804c9478542bb665b2d694d87a0437a44f13d49bbdcd7106f95b62b105478a4709bea251824a1058d082977ecfda03d257aafd460d893e9713d085a389be271e3162420802e7b8133e8f1f71dc38fa01c07a616c35b484e4abd791a6aecd9ece7d107a90d110754afe973b3d6b1bbf5a0e51e122b66863d3132ff63c7cacbb4d4449c788076e9f10f3d28de3ece92f5dea0521e7a7eee71a2d2855e898ba4a610b0fd6ce57bb4adf8e5d2634e033612aa5aa0c1401fa14ded62f7e1a695d505f4148beccd2be22bc25730ef75a5c94ff680cea0bda976ea47bd694d0cd116dc1aae5e56b8333b3d6b053ea8ded30e656725bfdea0e581d8d0e4b3b5ab0d441e1bec84eb205ab2f55ff06354dc47c3f53e3e91c79ba066a53d18e20d26fb3c45305e29d2cbd1ae40fe17f63eeae8ae1d2190a1d91fcda00df4111cf09b03686b76e192425627c394d7e75ac0dc96c6612366b8d68d3340a0c96d1de5da569fd8b08757d8393dca4a3000e001e674deda10d3072b1f8fff25a06ae859e5f232fd752442a5927c5433aaccd122971f59aca10c2ace8a46e5d9c5a0ad8a4773ddacac3510ee6c397f448fa9fd8f70b51d720d424313d6537f64b5eea0bc11c88bcccaf2086de82fe31e9fc2d225563c9252cd8a1b1660399627e642a980").to_vec()),
        BoundedVec::truncate_from(hex!("f90211a01b824a90c181a4d552fec1cc2ece9e933e9c4f6d266097bec54ae98b368a61e0a01f4fba7f84c2e7ddab72e34805dc74c8bb02a003a92e0387b9775ed2a6302d53a06b3e05fa2cd4b374baeeedadc843119323986042d2822b7feb0af7db72926097a0a2495430fe5d48192b266760828d497f3a3c1fefb8b100483c33341a3b039697a0e1b97d100eab7cf1ee2ad00ebd941ec784ef75d2f37dc13e0a8eff47439f7223a04464b9e147b9ed333e9fc1f89f7b9c0d26d4aef49c4430f2d4f2852652d9bb94a056326b52b65780b847eed732801e43ab8f023a6bbdda9c07da3bda25b7afc88ba05f02ed93551f04ec5a1d98da11483897b27f4d252d8d27a616b483b98021c24da08a956e1065bdc19ad7c20878b2579dad7ea1f862e411007f3b464f77ce01c192a06f48ee881f5a1f687f8c6f2d15d36493d1bfa04e4a32cf22080dac94b3de9f1ea0f516759cd854e7847023c7d7924f2986fafbe20eb3f601beed3b406b1ba9e2c5a04366281a689a29f9ed771d45ea5f3ee66f8a09df21f1849055203030c7cb7036a023a3e5edc04df1eddf74604f6660df52b1bf48f29a4b97710d2f87b57ed4464ca045bd1a9b6617bc253fc689e98d5000de2d065f707ac7467607e654ba6042762ea020b05480454d83862510dd3c4696719ae3c8554f09980bf49b0e3a1576c9769ea05ffa88eb9f0947617248a029b087e89c72bd1f1cc645f0ce38356b9f5d0f9ac480").to_vec()),
        BoundedVec::truncate_from(hex!("f90211a051b4b24ddc5e92b57fcb73192a8b66d9d0a682e1cc73d1a03272a9f4514b97d2a051b20ccb0db45c4f2a10588f5e8a27f5cd067394cdc16edfd4b808bae92a3534a0e1df6b6c81a8591be15c229e932f2edb46de5d80520308b952b757b55582a0dca0812a635747f148ed16fb42b8e4cf4d50db52270fe0e11cc7cb8cb92a0fc1b28ca03b098829ec412b08a1126da4495a8f755ebab82bc45ee43b92cdf4c36eac304ea06cd28b484d51aa518163bcc2d283d17833194168a3f11dcadf022410745c7e96a005b98558bfdd9059fdabd9f2ad3924ad005529a454438728b711ca76f858a3c9a0aec8e508057dd392fad463c3bfcca4b07206b151d53ca4a9876ae4a351663f89a0cbc58bc283fabbfa3018afac72b4b21d9a04a157efcfaea50b747f98e5836a53a00e26a1305e5039b385f353d4d89ede2f70b699ad262bc22eb88f7f22a43b87e4a0facb7e1f785a7056978aea57fa457856ba9771fb27f6950b6e36a6dac86335b6a0f257bb3e27acb6f1b0067832538536ec8cb6fa476184caf373169f64bcef398fa03b981c3a3b71e048c2217feed641e5501822d8ca73d07e12737ed4b178cd929ca0850c81bd936124ea2c3ccfd40be173599b2df58b0a5e5cddad9cbc40a6a6baa1a014b90165a798608acb60e89a66e3711d6beb3c65910e51bdcf6364803fd26747a0c2343b8c71ed71298edcc8bca4001250055bf1e3110d9f9e9bb2de2186ea666780").to_vec()),
        BoundedVec::truncate_from(hex!("f90211a0cb0cdddaf997332e6bde2eed1e92624ebfd39d1954ebd82f6ac02ea91e9d1dd6a07d022abe2fb0047caa61729dff7a72d087019d18037b4e9a38d6c00dd7f853bba054b2ccdbfd7c9f548068ad017ca4f3a9477afc713af9190d55d3ff3c868da7bca062912825979c5160c38e7e1d658a72d45956af648aabcd7e325adfcc955cd608a04c4e18220e36d6f10d20d6ea82b3023a61bb512981a52c3135115d00572fd9a9a09ca3e2a669bf7d2b24a635d874ffa5990c372e8e5abc8bbeeee15020b9b424b1a072781fd5d2c736c7986fb59b69c7ca355d268cd5a73ddf9202a05632fe42435ea08777c905af547074715fd074e3748bb0d138944b0a94adee42b70b64330e1816a0d6745d1404c2246391bd72db719ac915518a4b26deeb0f4640e6fd8dced6f2b0a0685d6698523489b73b28ac2b30568a78bea6bfd5b483b22316da7c976ab9fdbba01d30a49c428f3c3212ae5dee0b95a79f873f1fa344aa83b438886c0c507cb091a0c69962a4ebce43581445d16f514db993046127ef98294326cb45fdc5a42cb82ca0030e05160ec3744ce8e6d13c4801edef8169be649f13045b6d40d4e9b743807aa06253d30ad1389e78120a12498b69a0267a41d025dae18281d3aacaedd41f33cea08cae98ed5dd53ce4452b340c85f32a7e0409f9d295ecfa877e71305962bced6ea084af1765acd884a8ca25266477e22054558a6cbca0d68420b3df3f63ac19a06980").to_vec()),
        BoundedVec::truncate_from(hex!("f90191a0f9f2953d2c1c40f457ae2a6894201961f616a42ee0e942b338dfa7435800444ca0e67c195c9dcf9ce6c5bbe14f941178e1f88bf8c4b96c10fcdb94011ce241bf63a034c4c4a975771155c62835e2c8368b6d78388a45e597647cc8f7a704f9e1a5d580a009ed37b7f91327e4defae392a273c53541b8b5bd9ed76b8945b8ffff1a5fc0b2a005970b28afcc6892c47900bf9ae403ecd5364f79af62a0cdfa8ca4e04925dbdea0c6894ea8fb30775c1cca24dbaee5365e3d3f3df625736c0784378190d4507d8580a0f840f68a60355397bb24369387aaaa776432dceb3db4da14de72d82db76d00f7a0bdba75a92da491a76173558d6e07dc24699534af0be478b0bb803acc76251d5780a064dc4f711ccf12b28b6304ee111d7ae5edc34ffca6688c0f25846edcf1b3468ca0a285ed0e46b78847cf1d0fd0c8b81580e9417a534357d6bd8b9496f9ccc529b7a0abc253bb447ace56fd185b004d2764e61fffb6bd21ba278c1e0a0864c35080a3a0184573b58579fcde692191ba18c0b30dd06326129c3861bf55d29737f646162a8080").to_vec()),
        BoundedVec::truncate_from(hex!("f8918080a0f2054bc8fc8c37e14cf4934ff82e49f5016007cf3c8a16d5699d04de84b692eb8080808080a0e53905b3f1ea5db82f82f442b71d07c6a97cfa5ee491e30f850be0ea3389466ca062f6d91b16d4d417f8643b7315ace6e05afb70aa59c0b19fc863c33f8c87100780808080a0cf5bc8e1a57bb56b9c38250e75864a1fd7a893ebecd6bb0ed2ed301c68d581a18080").to_vec()),
        BoundedVec::truncate_from(hex!("f8669d343c360e5679746f94d3583df82fee3a96e5a6b33dcf5e57421b01c9a8b846f8440280a0089abb6beda7a92401ee554da94e735920984a64e7403d5cd7c82098de059da9a054ccfa149cf4cf471e53bc5dfbfe8ae5c27fa96b0599f2747ffe6e1fb8bd4396").to_vec()),
    ])
}

fn get_valid_storage_proof() -> ValidProof {
	BoundedVec::truncate_from(vec![BoundedVec::truncate_from(hex!("f90131a02aa432bd6022e7da6d0188f41427645658b74a08227a036f80fa44bff9fc57af80808080a0c01b2d5e61b71c73bc7b1b73db39a4d89bea07f4594557fec2242da9c90c91a580a073768754f5d28b5a762b44f6c37233fbedbeab05779b7dfb407ede56d2812891a01f3fcf34e7a10de63f5fe300d0522504aa29e6a4a7ad39df02bfa94e958e5e458080a08f056b51124e5c81f7d86b4364a97f66324957dc1042f751bce474abfa1480e5a098b414db83b4c1efa80ad36b641c9c517b2225cb212fc236116b2f9dc69ce64fa09a0e693399670076dd7708930bdc19b9101b3f181c4bf62d25edf6e1fd6e881da079866ac4ff54c3062d8fbd4fa347961e9a905b4114a2ed9785e22a5c03f4ffb8a03a1e44d3669992d3ac3f750a1159442ba1d83432cb03e5815d7eed7d97c7e90380").to_vec()),
                                   BoundedVec::truncate_from(hex!("f851808080a08ddae10e810d2127e5d527cab77909cdc5b99e20544edfb6f0c7b46033fe17e080808080808080808080a0c43b94283974430fdacaadaea093cd1524e306af5fc8c15ae3b13528d8f332088080").to_vec()),
                                   BoundedVec::truncate_from(hex!("f843a020b5be412f275a18f6e4d622aee4ff40b21467c926224771b782d4c095d1444ba1a0efac9989593dfa1e64bac26dd75fd613470d99766ad2c954af658253a09d1ad8").to_vec()),
    ])
}

fn get_valid_amb_account_proof() -> crate::ValidProof {
	BoundedVec::truncate_from(vec![
        BoundedVec::truncate_from(hex!("f90211a00b9369252d91d5240966845819a5e73a8e7b4b531b9f66bd652df2ef25999e8fa00fbedaed13519e0d8971ca3e8f54fa6250b8c60ae522924ea5b7bc54d7ab049ca0e9a41917172ecd79fbbc0db3c117a990464df2767be84bc5fd749cb50763834fa0420cf2bd2e7aac8cc33b0731d7546b738315ac482524aebc60e39a7d0359ce84a007e78eae37bbc3d25305685e8321582390d9631365f5c40612d111c7c1d2a35aa0f90094ebbf4d71dc00d9ff469a149d62529289b7a88d63f69ef73c45ffadd9b8a0bb5c4d709b179a0dec8c3e298249746c5f36f1ab4c7bb0985e4c565bfa96efd2a0b44d9462647f7f99afe5396fc6c049ca3d343748d9eb27f64e118a4d55d26d2ba08119035469c78ffe7336ba8c0117906ca64c9827f16ef71b5fcd99272d983779a067a02fa9917bda0acfb3646ba7e91c7e54e56e40342c552c0b069f949a64e122a0d2b39ca49bbaf2582e8c48243c86249ef034d14f8272952975c1f01773d508c4a0cd89136cd6f4203f52553a6856ba48ee08fe3d0165b5d020b4781ec0b0195bfda0bfdc0af814301e39e11e3c2186b3969b5c6ffdcb627d99f2e1e425d48300253ea0689a72bc1f0084b35849c938c2298e0d90384a071b6ef7bb145dab1b1205efffa0614a4f66f2451debf003b8584d9c2bbb87ee1d141cb240b494a840a91d32e126a01885c00eb4d6365b341f3c21bc44754f901ff56e957399069209b612c272a97880").to_vec()),
        BoundedVec::truncate_from(hex!("f90211a0f965da070dc0173a61d34b21bea4dfde26c834257629677157ae7248b50cd81fa03a76449b49477746172ed14e2f996a8131b3e23de3315a1923f38b7ea415b979a0d074384d170d5abab0c1d9ef28cc3c0ff8f36851a7bcbaef501c6032df0943aaa0149b7581ab9bc820217566d4304c13c418893863964f1b2290390f2f4f9c07baa067bc808de655c5710908cbb592420df656ddb294a0ca123c5d1c074e977175aaa02965ae45e7ec302c8d1d24d07954074343df8247da2f6122939181ca3f187dd0a0edbf4144f7d295047f49dd10e218aa3d590cb406312310b0ef56184c1d046822a0915dbafe8ba1d369a3e24375095596455fbc9aacc468958f6928ce428ac31967a0b9e44b513ed8ca8ee7409e399c4a467dc7d260c28d1fba4685852f5e3b65f4d8a09567dcd6366635567c2e61fe0d984516a77a4b0c204bd9b2f85ff343216780afa08390c69ab6052b335f241b5b187af412395910213899dbbf6084987846204c7ba0d38a47f0e0b2b6267a7e360308cbe0ab49e22a0a8615439d3423d6c363080ff9a083e7617b1a68407db284cd4035e04b0637b727c949205cd9ed176319cb9c0e52a061503e01e7f1d1fa6ef925e561be0a4a6fd580263da8f811121b90b87708e562a09c0414ab3be0fee449c042627dac5fd0f3d228b53499a0fc181045f873e3403ca092962b27d984b35f646fa84d3ef49830c5ee8fb0d6db97a912aaf21da043187580").to_vec()),
        BoundedVec::truncate_from(hex!("f90211a0b882b0bbc9b8e3d25a4829a3ce4d49ec7c784383a50f5ff5aa49afeb1ff11cbaa03a6a3554903a3fc21c4026084a3170c202a1c64230dc99b13fdf14985a28183aa057931ffba973f857ce9ff8fbb84958b8e4cf1477edd321e757002870535e308ca074c4acaec7053c4b57a5916049ee1c48b6177badf67cd10c76130a1b8c519329a09c64edf3c38e2de3724fcbae6235accfa9025f9cc0e15abd79d534094b4405aaa092a8e4d137cd6795a50516a76d7098981f1cbdf767fc7cf53f807d32b5101b3ba0c3db2eb22e70e19898ea197ff287371c8a98526f1fab57b446a9082abd5e77fea05cc5337564cd40ceb6f21e1529d46808a7f9dfd03702ed6ccaf31be9a20baeb1a0f453e0497c3ffddaae16012a300dece9147aed514e66316b603a8b438422011ba0fda6a467384c1277311ae9586e5baa65e3fb197cab01a9d839b81b5343f19290a00f02720543a48e733ca4f18777471b0840f1f25dc0c8e61de5715504903609e6a05c825f0b4465760d160e47f5f84e736c7b0d2196e52a01c3974b125fd7a3224ea04d04067ea381da30cadfd36aa2cb1804e8307254bc9a883ebfa39d159b61c119a07f829aa1f376ffe4373e40912576beaf801b035fade0eb9923aae9d2d4cdc4cea092664e0322a11173fa7da7d5149d9b042551e546f9cb9912e86db84b7a6f8ee2a0711815f7bc0dd6bbe61bebb5c93e8a0f5e07b193fe303acb318e4ef8ebd74a9380").to_vec()),
        BoundedVec::truncate_from(hex!("f90211a0fa16c381c9407da90879bf32e2084769a102f4dd505cee8093a3f5f5d5d11897a02e7daca78cdd4bf6f895f54c08da29ec71c621dc987178f71cf88837422b9dbba027bb1b0aaaaec895c5f719e485905eda3119f4df0f3ba4fb53e560974d17f567a08ef21213d4f2b18f4afbf5e1a3823161a31971d18d0b83c6df044cb6676b7ceda03d9ab8d58ef85e067b9da2bc899df09dcd41a886076521fa8ee3afb25f13d7c2a06d355255fa840d7aca803bb9486a969879ef3f403facb0b38ee3e7c89255e043a01a289c3af5517fd34799b37159c365964ed3511031574af034788c76d7597bb7a09de99bcbf90daa9fbc363cc42606f1d303d3589c36e1a037758fed92a0186d51a0c6ee166b4fb46bf4272f5fb3d2947649db5042e999a4b1a579d781151797121da0ce52b5820413a2bf7de720841e6a7224857f9adcf5dea56fa443c0675264931fa0975c99cfe1455d01ee6f3199e30a4d60f0770588454a235956f114fe9a4cac3ea03ad3c23b37f0004e835b8f2e0435c0248fd039c20c33ddfbbd7541b891b4afaca0f93e9f7841ced51de169d29613da8e39dda1c486bf03674fe25c74a294a57675a03a1aa0730d32cc21169724a4cdbd4f2b97245e520814a8983d2752e15455238ba0dc1eb94acdc6fa937ca3cfe76a5d7f40935beea7d6bcf0ae0b9f95f9b96abc4da0b4ad959ea31afa024dd49a00c6c8b83b7192bd05c9e30d3382d6d47fcbe7f10280").to_vec()),
        BoundedVec::truncate_from(hex!("f90211a0480f6762979b1760658dfb6e799619c1782c056285565d909532cffdca48a7aca09722f0a6e84b4709d9d679709cf56c36030401959a68ca1200fca4fd45a7dd38a000835e044809c0a8e9ba3b91d2ff851051480178da4ec312c61aec9a246d3979a0669269114e17b007ea67b3fbf27aa2f84fd2c8003ce1f36b8d33d67a6eca7469a0171cdadb913b5d039f776caceaa312bbc8e8dc357e7d782dc3a0c6e6a2a0da5ba0eac5dec43c8a7a00e40aecbb0d0b13f31464ff1ad4a764c6ff4581cd6f2a7332a0df68af5b5502dd5ef713e5f670cb11950e713ae8c143a4f331b05bf5dfdadffca0215883bf3ae115539f7792c3187a21ba302928b2184e81b05b5594a917018774a0e4c394a136f0c0e1c9a5bf73f249f6f7df3b664323a302924bb5ba269f7ee61fa0035a6b61964189a1508e2ad03d910e90dfc995b8aaec8b7a43bc460c72a46606a08b4a9bc6de38ca5e2405d1dd2cf8709bdb4b92e969210bef2a89ab286799bcf3a053d23a5fddd8218e1c5d3664496e09dfa4e4ebe513014d05ab949558c18399fea03949393cafa9086ecc8f55516cd414034deb32b32e35bcf7c75c31116a544008a0d5c4b0586d91d02284d2343033f5b65572b4ec07b61e650bf0d15049bdd00ab0a01c4f252dd383c4d008e66839589716fece51f9748a761dbe2291c008896a2e28a0ec1f09945d1f38964901d79792ee69ae5300bb7cee5a5bdf6d9e6efdb8eec43080").to_vec()),
        BoundedVec::truncate_from(hex!("f90151a023b80573a6326250c3ae2efbe4af8207d8c1a20acfbfbb8444c7b728bff4e054a0aace3888a6f1f350b8d8ebc1e11687281610dd304eb527cbf884e12363a5a7a3a07cc000006215a0e9d69011c9c53a2b1ba6412a2b98c0439c8efde0a55601b58ca045d4fb94af2b968111c95fc4e55d1da32159431cf5e8b7d99334b90a8cb0efbd8080a00b092b863d6b5435d71a0d65ece895d7b115b9d6d76cd633def212627fe50dfc8080a0270c83c653946af43045281ed905ad9307ba019729f6b8c4712a30ce3b2b12918080a0c0efdb94ce26fd150d90d049e10595cac38e8f151198536fd9d4f8965ca772ffa006a6f61b1e3d71530b9b630d2fa306a480b49bdf83e14ac397398cf5f81c7e8da02acbd28e4f5185b16c60ce005bcb30acaf8e3e70e46db714869bb0f60e1a3a05a0db46a0c8fb8a7ed852f1fcffd35af11aa2bf758ffa8079ac1739d7c33e23511180").to_vec()),
        BoundedVec::truncate_from(hex!("f87180808080808080808080a0ab0da2d5bc9713d2a60975c6fdd916ba1729acc7e75110677583e7e37c2c6df68080a0c186d6514c8893dbbde6a0d9b5294857892ce0b49a44c0799edfba1567b0961080a0c37fb0f6ba6981de5e302ac5538f212162e0b885068c9f1d9ebe091c1d4e031d80").to_vec()),
        BoundedVec::truncate_from(hex!("f8669d32f0e5aa610617dbfcec97bc206d49dc85c62fdfcd084b1fec6dd67dc5b846f8440201a0dad0d599c05e47b3960c18fcb0ad5089027159340e6a86e62a1f618945433650a0636d977dc571e5f18377d0f398751d5ca05b1c5041b6bc97fc3564217d52ee53").to_vec()),
    ])
}

fn get_valid_amb_storage_proof() -> crate::ValidProof {
	BoundedVec::truncate_from(vec![
        BoundedVec::truncate_from(hex!("f90131a0358aed89bfaa22b0179d1015fa6034c7ad29702af20b05b02f3cf63f69c2811280808080a08c857f53d31e0c1b681ff481d435a48986faa0615d611b05db5d0b3f4691f6d1a0d40ba3e2c5ca0722bb231d4a661b452920758ab202dbcfbc8e505d9f436e5600a0c3d20e38630b6b8f49ea6ca919b9077081557a4ca337c374e42219e4738a8970a0d2059c3c7e4dacb02dec49822ce85568a4419849a093ec956598dc2c3269238d808080a08c675371a85fc8f524a707007b05d0b87d7ecbde37f10e953a03da38431d31f8a0079a6b40f411dbc043c20594864f4e7f3e5ef2ef8e27f230c3431ec771b2d001a079866ac4ff54c3062d8fbd4fa347961e9a905b4114a2ed9785e22a5c03f4ffb8a0fef4138a6a9993fb0418e252583a03d2586caf404c7a16f7083600f49aac8cf280").to_vec()),
        BoundedVec::truncate_from(hex!("f851808080a0aec544652aa67b55271eec87a45f5ca89f6a6ea762450ca63b014ceb073e4e9d80808080808080808080a08c06dc4d3d3e8d7fe5a8a88222594ba9f4cdb19baaa8e60919b5617770423f828080").to_vec()),
        BoundedVec::truncate_from(hex!("f843a020b5be412f275a18f6e4d622aee4ff40b21467c926224771b782d4c095d1444ba1a05774ba3f9618e2da3885b0e2853e4005c3e836625e8be0f69bf3d93f51fac58d").to_vec()),
    ])
}

fn get_valid_step_input() -> FunctionInput {
	BoundedVec::truncate_from(
		hex!("0ab2afdc05c8b6ae1f2ab20874fb4159e25d5c1d4faa41aee232d6ab331332df0000000000747ffe")
			.to_vec(),
	)
}

fn get_valid_step_output() -> FunctionOutput {
	BoundedVec::truncate_from(hex!("e4566e0cf4edb171a3eedd59f9943bbcd0b1f6b648f1a6e26d5264b668ab41ec51e76629b32b943497207e7b7ccff8fbc12e9e6d758cc7eed972422c4cad02b90000000000747fa001fd").to_vec())
}

fn get_valid_step_proof() -> FunctionProof {
	BoundedVec::truncate_from(hex!("0b496d04c0e12206bc846edd2077a20b8b55f65fc0e40bb8cf617d9b79ce39e508281ad49432300b3b7c8a95a0a63544f93f553fcfdeba38c82460888f4030ed1f67a1be666c12ee00658109c802042c58f645474fcee7d128277a4e35c1dd1504d33cb652ec23407cd3580eda0196dd97054eb5c2a817163d6997832d9abd422729b3e85a15941722baeb5ca8a42567a91c6a0b0cd64ac15431fde05071e90e0d30c12013d5803336cc2f433c16eaa5434e30b89ce7395c3c3cda29dde3be062281095f143d728486c71203b24fa6068e69aabf29d457ffadc6d682d51a4f08179d3240bc561ae7e2c005bb772a4d4c5ba6644986052fad554f042ab0074a8f").to_vec())
}

fn get_valid_rotate_input() -> FunctionInput {
	BoundedVec::truncate_from(
		hex!("e882fe800bed07205bf2cbf17f30148b335d143a91811ff65280c221c9f57856").to_vec(),
	)
}

fn get_valid_rotate_output() -> FunctionOutput {
	BoundedVec::truncate_from(
		hex!("2441c10b0b6605985c56ebf6dc1ca7e9a0ae20e617c931d72f2ec19aa40ccc8d").to_vec(),
	)
}

fn get_valid_rotate_proof() -> FunctionProof {
	BoundedVec::truncate_from(hex!("14305744fb26a377656a947cae0874c14b086de9d407bdfaf415ca9f47402c04144589183b473537750e7211f93671e324825db673edcf5c0839b08eecba08202966ba52dc07e1bf9832a54770048b84999172d47c57628758d8fe43dd9fe1412e6f8c0e75a79cde28e0e24eb09f9d23309defb07f4a1761deb6598de77278971d2d914930ad2e3ad8b6264e595a0516a912fc9394c93fa61146efc54d61e5c32378a5d4460aa2164422702f9401fcfb3e2b991a0e5b847ede3ea9ffe70a55100203abc0636c101adb6546c2f7aaf32d79e69093afb40c3c1a674e44a1ece76a1183fc03ef9553a7728672de2aada5d5582b5bcf0859e8c312ab59429553ed6d").to_vec())
}

fn get_valid_sp1_proof() -> ProofInput {
	BoundedVec::truncate_from(hex!("11b6a09d118ac1f4d588d81a97770ff506c49ec0a7819e2cee1d043f76b381b16b4795ea249ad3f7aaccbffcf499a91d1bce5851383c9bf7a75bb95408e5c6bebc3cb8fc2413e6b5c527d5907e89a81cd8a17987208164cc88250db8720d8375a13291fc24f583f49d19d2f40afffd59fb65c40ad32e96fe37ef151150881d7992e039ff0d51180710dbfa3d2dfe87de458cba382ff125da0c3f1855bef9d29488b41c312897b742b907b9c14c04359e8b874fa260a27dc4237e48b0cbe7749b76723813011208a6a9cc0e33d4f027af7a3bfd22622816a4497748da7f7a1a1cc286abcc00e228f365edd0770d67ca3b0d5e0f5f536fff6ebb207fcdb8688af762aff9db").to_vec())
}

fn get_valid_public_values() -> PublicValuesInput {
	BoundedVec::truncate_from(hex!("711c632bedcfb9b85e6dc1bd53ef7c69753f5836761c69be029b55b8097fd6145a2d22ca66c5c60b6a181d25ea9f54b8669440ef59f71a8caf818b00bbd34c995aaad4aeaf4c830b38bfe4f9f5fe3cfcdad919639b871ea20cf1e7eff30a8a360000000000000000000000000000000000000000000000000000000000669d00d7bd59d9998ade89e28efdd86630633bf71117417aa107163526b8d748d332840000000000000000000000000000000000000000000000000000000000669ce0cfde56afdd0992f3fc0a576c2b8959f66a88d47f2886046c104fe6e14da386ac").to_vec())
}
