use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_groth16::{prepare_verifying_key, verify_proof, Proof, VerifyingKey};
use ark_std::boxed::Box;
use ark_std::str::FromStr;
use ark_std::string::String;
use ark_std::string::ToString;
use ark_std::vec;
use ark_std::vec::Vec;
use codec::{Decode, Encode};
use ethabi::ParamType;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{H256, U256};

use crate::state::{CircomProof, PublicSignals};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode, TypeInfo)]
pub enum VerificationError {
	InvalidProof,
	InvalidVK,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode, TypeInfo)]
pub struct Verifier {
	pub vk_json: VerifyingKeyJson,
}

#[derive(Debug)]
pub enum VKeyDeserializationError {
	SerdeError,
}

/// VerifyingKeyJson struct that contains key for Rotate and Step verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode, TypeInfo)]
pub struct VerifyingKeyJson {
	#[serde(rename = "IC")]
	pub ic: Vec<Vec<String>>,

	#[serde(rename = "nPublic")]
	pub inputs_count: u32,
	pub vk_alpha_1: Vec<String>,
	pub vk_beta_2: Vec<Vec<String>>,
	pub vk_gamma_2: Vec<Vec<String>>,
	pub vk_delta_2: Vec<Vec<String>>,
	pub vk_alphabeta_12: Vec<Vec<Vec<String>>>,
	pub curve: String,
	pub protocol: String,
}

impl VerifyingKeyJson {
	pub fn to_verifying_key(&self) -> Result<VerifyingKey<Bn254>, VerificationError> {
		let alpha_g1 = G1Affine::from(G1Projective::new(
			str_to_fq(&self.vk_alpha_1[0])?,
			str_to_fq(&self.vk_alpha_1[1])?,
			str_to_fq(&self.vk_alpha_1[2])?,
		));
		let beta_g2 = G2Affine::from(G2Projective::new(
			// x
			Fq2::new(
				str_to_fq(&self.vk_beta_2[0][0])?,
				str_to_fq(&self.vk_beta_2[0][1])?,
			),
			// y
			Fq2::new(
				str_to_fq(&self.vk_beta_2[1][0])?,
				str_to_fq(&self.vk_beta_2[1][1])?,
			),
			// z,
			Fq2::new(
				str_to_fq(&self.vk_beta_2[2][0])?,
				str_to_fq(&self.vk_beta_2[2][1])?,
			),
		));

		let gamma_g2 = G2Affine::from(G2Projective::new(
			// x
			Fq2::new(
				str_to_fq(&self.vk_gamma_2[0][0])?,
				str_to_fq(&self.vk_gamma_2[0][1])?,
			),
			// y
			Fq2::new(
				str_to_fq(&self.vk_gamma_2[1][0])?,
				str_to_fq(&self.vk_gamma_2[1][1])?,
			),
			// z,
			Fq2::new(
				str_to_fq(&self.vk_gamma_2[2][0])?,
				str_to_fq(&self.vk_gamma_2[2][1])?,
			),
		));

		let delta_g2 = G2Affine::from(G2Projective::new(
			// x
			Fq2::new(
				str_to_fq(&self.vk_delta_2[0][0])?,
				str_to_fq(&self.vk_delta_2[0][1])?,
			),
			// y
			Fq2::new(
				str_to_fq(&self.vk_delta_2[1][0])?,
				str_to_fq(&self.vk_delta_2[1][1])?,
			),
			// z,
			Fq2::new(
				str_to_fq(&self.vk_delta_2[2][0])?,
				str_to_fq(&self.vk_delta_2[2][1])?,
			),
		));

		let gamma_abc_g1_result: Result<Vec<G1Affine>, VKeyDeserializationError> = self
			.ic
			.iter()
			.map(|coords| {
				Ok(G1Affine::from(G1Projective::new(
					Fq::from_str(&coords[0]).map_err(|_| VKeyDeserializationError::SerdeError)?,
					Fq::from_str(&coords[1]).map_err(|_| VKeyDeserializationError::SerdeError)?,
					Fq::from_str(&coords[2]).map_err(|_| VKeyDeserializationError::SerdeError)?,
				)))
			})
			.collect();

		match gamma_abc_g1_result {
			Ok(gamma_abc_g1) => Ok(VerifyingKey::<Bn254> {
				alpha_g1,
				beta_g2,
				gamma_g2,
				delta_g2,
				gamma_abc_g1,
			}),
			Err(_) => Err(VerificationError::InvalidVK),
		}
	}
}

pub fn str_to_fq(s: &str) -> Result<Fq, VerificationError> {
	let fp = Fq::from_str(s).map_err(|_| VerificationError::InvalidVK)?;
	Ok(fp)
}

impl Verifier {
	/// Creates `Verifier` from json representation.
	pub fn from_json_u8_slice(slice: &[u8]) -> Result<Self, VKeyDeserializationError> {
		serde_json::from_slice(slice).map_err(|_| VKeyDeserializationError::SerdeError)
	}

	/// Verifies input based on the supplied proof and hashes.
	pub fn verify(
		self,
		input_hash: H256,
		output_hash: H256,
		proof: Vec<u8>,
	) -> Result<bool, VerificationError> {
		let input_bytes = input_hash.to_fixed_bytes();
		let output_bytes = output_hash.to_fixed_bytes();

		let decoded: (Vec<String>, Vec<Vec<String>>, Vec<String>) = decode_proof(proof)?;

		let circom_proof = CircomProof::new(decoded.0, decoded.1, decoded.2);
		let proof = circom_proof.proof()?;

		let mut input = vec!["0".to_string(); 2];
		input[0] = U256::from_big_endian(output_bytes.as_slice()).to_string();
		input[1] = U256::from_big_endian(input_bytes.as_slice()).to_string();

		let public_signals = PublicSignals::from(input);

		let result = self.verify_proof(proof.clone(), &public_signals.get()?);

		result.map_err(|_| VerificationError::InvalidProof)
	}
	fn verify_proof(self, proof: Proof<Bn254>, inputs: &[Fr]) -> Result<bool, VerificationError> {
		let vk = self.vk_json.to_verifying_key()?;
		let pvk = prepare_verifying_key(&vk);

		let result = verify_proof(&pvk, &proof, inputs);
		result.map_err(|_| VerificationError::InvalidProof)
	}
}

/// decode_proof decodes proof into points.
#[allow(clippy::type_complexity)]
pub fn decode_proof(
	proof: Vec<u8>,
) -> Result<(Vec<String>, Vec<Vec<String>>, Vec<String>), VerificationError> {
	let decoded = ethabi::decode(
		&[ParamType::Tuple(vec![
			ParamType::FixedArray(Box::new(ParamType::Uint(256)), 2),
			ParamType::FixedArray(
				Box::new(ParamType::FixedArray(Box::new(ParamType::Uint(256)), 2)),
				2,
			),
			ParamType::FixedArray(Box::new(ParamType::Uint(256)), 2),
		])],
		&proof,
	)
	.map_err(|_| VerificationError::InvalidProof)?;

	let mut a0: String = String::new();
	let mut a1: String = String::new();

	let mut b00: String = String::new();
	let mut b01: String = String::new();
	let mut b10: String = String::new();
	let mut b11: String = String::new();

	let mut c0: String = String::new();
	let mut c1: String = String::new();

	if let Some(ethabi::Token::Tuple(t)) = decoded.first() {
		if let ethabi::Token::FixedArray(ar) = &t[0] {
			if let ethabi::Token::Uint(u) = &ar[0] {
				a0 = u.to_string();
			}
			if let ethabi::Token::Uint(u) = &ar[1] {
				a1 = u.to_string();
			}
		}

		if let ethabi::Token::FixedArray(ar) = &t[1] {
			if let ethabi::Token::FixedArray(arr) = &ar[0] {
				if let ethabi::Token::Uint(u) = &arr[0] {
					b00 = u.to_string();
				}
				if let ethabi::Token::Uint(u) = &arr[1] {
					b01 = u.to_string();
				}
			}

			if let ethabi::Token::FixedArray(arr) = &ar[1] {
				if let ethabi::Token::Uint(u) = &arr[0] {
					b10 = u.to_string();
				}
				if let ethabi::Token::Uint(u) = &arr[1] {
					b11 = u.to_string();
				}
			}
		}

		if let ethabi::Token::FixedArray(ar) = &t[2] {
			if let ethabi::Token::Uint(u) = &ar[0] {
				c0 = u.to_string();
			}
			if let ethabi::Token::Uint(u) = &ar[1] {
				c1 = u.to_string();
			}
		}
	}

	Ok((
		vec![a0, a1],
		vec![vec![b01, b00], vec![b11, b10]],
		vec![c0, c1],
	))
}

/// encode_packed implements abi.encodePacked function for poseidon hash.
pub fn encode_packed(poseidon: U256, slot: u64) -> Vec<u8> {
	let bytes: &mut [u8; 32] = &mut [0u8; 32];
	poseidon.to_big_endian(bytes);
	let slot_bytes = slot.to_be_bytes();
	let mut result = bytes.to_vec();
	result.extend_from_slice(slot_bytes.as_slice());
	result
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;
	use hex_literal::hex;
	use sp_core::{H256, U256};
	use sp_io::hashing::sha2_256;

	use crate::verifier::{decode_proof, encode_packed, Verifier};

	#[test]
	fn test_zk_step_with_serde() {
		let vk = r#"{"vk_json":{
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

		let v = Verifier::from_json_u8_slice(vk.as_bytes()).unwrap();

		assert_eq!("bn128", v.vk_json.curve);
		assert_eq!("groth16", v.vk_json.protocol);

		assert_eq!(
			"20491192805390485299153009773594534940189261866228447918068658471970481763042",
			v.vk_json.vk_alpha_1[0].as_str()
		);
		assert_eq!(
			"9383485363053290200918347156157836566562967994039712273449902621266178545958",
			v.vk_json.vk_alpha_1[1].as_str()
		);
		assert_eq!(
			"6375614351688725206403948262868962793625744043794305715222011528459656738731",
			v.vk_json.vk_beta_2[0][0].as_str()
		);
		assert_eq!(
			"4252822878758300859123897981450591353533073413197771768651442665752259397132",
			v.vk_json.vk_beta_2[0][1].as_str()
		);
		assert_eq!(
			"10505242626370262277552901082094356697409835680220590971873171140371331206856",
			v.vk_json.vk_beta_2[1][0].as_str()
		);
		assert_eq!(
			"21847035105528745403288232691147584728191162732299865338377159692350059136679",
			v.vk_json.vk_beta_2[1][1].as_str()
		);
		assert_eq!(
			"10857046999023057135944570762232829481370756359578518086990519993285655852781",
			v.vk_json.vk_gamma_2[0][0].as_str()
		);
		assert_eq!(
			"11559732032986387107991004021392285783925812861821192530917403151452391805634",
			v.vk_json.vk_gamma_2[0][1].as_str()
		);
		assert_eq!(
			"8495653923123431417604973247489272438418190587263600148770280649306958101930",
			v.vk_json.vk_gamma_2[1][0].as_str()
		);
		assert_eq!(
			"4082367875863433681332203403145435568316851327593401208105741076214120093531",
			v.vk_json.vk_gamma_2[1][1].as_str()
		);
		assert_eq!(
			"677302577815076814357170457144294271294364985082280272249076505900964830740",
			v.vk_json.vk_delta_2[0][0].as_str()
		);
		assert_eq!(
			"5628948730667472013190771331033856457010306836153142947462627646651446565415",
			v.vk_json.vk_delta_2[0][1].as_str()
		);
		assert_eq!(
			"5877290568297658003612857476419103064356778304319760331670835003648166891449",
			v.vk_json.vk_delta_2[1][0].as_str()
		);
		assert_eq!(
			"10874997846396459971354014654692242947705540424071616448481145872912634110727",
			v.vk_json.vk_delta_2[1][1].as_str()
		);
		assert_eq!(
			"202333273032481017331373350816007583026713320195536354260471885571526195724",
			v.vk_json.ic[0][0].as_str()
		);
		assert_eq!(
			"8246242704115088390751476790768744984402990892657920674334938931948100192840",
			v.vk_json.ic[0][1].as_str()
		);
		assert_eq!(
			"12901454334783146822957332552289769626984444933652541503990843020723194328882",
			v.vk_json.ic[1][0].as_str()
		);
		assert_eq!(
			"12436078488518552293095332739673622487901350475115357313978341690183990059269",
			v.vk_json.ic[1][1].as_str()
		);
		assert_eq!(
			"12828056956769114977702246128118682473179646035440405756936949778100648490262",
			v.vk_json.ic[2][0].as_str()
		);
		assert_eq!(
			"7351319165217643779735289066901404053730163225836026220896225559268517203790",
			v.vk_json.ic[2][1].as_str()
		);

		// https://platform.succinct.xyz/explorer/426c9f2d-0b72-499a-83c0-f258d7d8c84d
		let inp = hex!(
			"0ab2afdc05c8b6ae1f2ab20874fb4159e25d5c1d4faa41aee232d6ab331332df0000000000747ffe"
		);
		let out = hex!("e4566e0cf4edb171a3eedd59f9943bbcd0b1f6b648f1a6e26d5264b668ab41ec51e76629b32b943497207e7b7ccff8fbc12e9e6d758cc7eed972422c4cad02b90000000000747fa001fd");
		let inp_hash = H256(sha2_256(inp.as_slice()));
		let out_hash = H256(sha2_256(out.as_slice()));

		let proof = hex!("0b496d04c0e12206bc846edd2077a20b8b55f65fc0e40bb8cf617d9b79ce39e508281ad49432300b3b7c8a95a0a63544f93f553fcfdeba38c82460888f4030ed1f67a1be666c12ee00658109c802042c58f645474fcee7d128277a4e35c1dd1504d33cb652ec23407cd3580eda0196dd97054eb5c2a817163d6997832d9abd422729b3e85a15941722baeb5ca8a42567a91c6a0b0cd64ac15431fde05071e90e0d30c12013d5803336cc2f433c16eaa5434e30b89ce7395c3c3cda29dde3be062281095f143d728486c71203b24fa6068e69aabf29d457ffadc6d682d51a4f08179d3240bc561ae7e2c005bb772a4d4c5ba6644986052fad554f042ab0074a8f");

		let result = v.verify(inp_hash, out_hash, proof.to_vec());

		assert_ok!(result.clone());
		assert!(result.unwrap());
	}

	#[test]
	fn test_zk_rotate_with_serde() {
		let vk = r#"{"vk_json":{
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
            "7118041338407665643077665093375077236507031390654037220453830314560753892708"
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

		let v = Verifier::from_json_u8_slice(vk.as_bytes()).unwrap();

		// https://alpha.succinct.xyz/explorer/bee3b89f-682c-4a18-b070-4de55f0e1d4b
		let inp = hex!("e882fe800bed07205bf2cbf17f30148b335d143a91811ff65280c221c9f57856");
		let out = hex!("2441c10b0b6605985c56ebf6dc1ca7e9a0ae20e617c931d72f2ec19aa40ccc8d");
		let inp_hash = H256(sha2_256(inp.as_slice()));
		let out_hash = H256(sha2_256(out.as_slice()));

		let proof = hex!("14305744fb26a377656a947cae0874c14b086de9d407bdfaf415ca9f47402c04144589183b473537750e7211f93671e324825db673edcf5c0839b08eecba08202966ba52dc07e1bf9832a54770048b84999172d47c57628758d8fe43dd9fe1412e6f8c0e75a79cde28e0e24eb09f9d23309defb07f4a1761deb6598de77278971d2d914930ad2e3ad8b6264e595a0516a912fc9394c93fa61146efc54d61e5c32378a5d4460aa2164422702f9401fcfb3e2b991a0e5b847ede3ea9ffe70a55100203abc0636c101adb6546c2f7aaf32d79e69093afb40c3c1a674e44a1ece76a1183fc03ef9553a7728672de2aada5d5582b5bcf0859e8c312ab59429553ed6d");

		let result = v.verify(inp_hash, out_hash, proof.to_vec());

		assert_ok!(result.clone());
		assert!(result.unwrap());
	}

	#[test]
	fn test_decode_proof() {
		let proof = hex!("1332c772a8f9a02f304b5472d3b6b75f1a494bd9b137fc663fd5b9b475992bc829ba08f7cfa745e340938e356b139224d0288b9511a5cec83235f969f61a94ed16a14579fa0adcc3bf8da36209f64547fd5ff4e1c7e8b5b151335b5b4a471de3115f83b696517ac68ae7620f7d3840e44aff4781c0a4d265a2905ef9bcaa04432a660197790e60d1135946ae0603ef69a5ecb45b6f8046167f902dc6d8a35cf716bce116484dfa4fcd5d8f4c2fda26d68754b56e68f1a877d95dc171accc34d71285068693fe3d8d28e66342c31292ceee5c6d87fcb8ad8c132363565f2aeff905726b2d35def5c9636dd5ec402d8d6f6c9a7be7977e7e5727da327ea5b079ad");

		let decoded: (Vec<String>, Vec<Vec<String>>, Vec<String>) =
			decode_proof(proof.to_vec()).unwrap();

		assert_eq!(
			"8683663015073067038244847214283351810649000192281314413199884219842452597704",
			decoded.0[0]
		);
		assert_eq!(
			"18873522240908759015197166908776808810045074443031598381394130502027574940909",
			decoded.0[1]
		);
		assert_eq!(
			"10235824555245197129038838261179705064387070473723531210466639418098968894947",
			decoded.1[0][1]
		);
		assert_eq!(
			"7858077948381560609212308446029826533408997041182829878371843519718814778435",
			decoded.1[0][0]
		);
		assert_eq!(
			"19177369026551579179894492468331397687405155911290633487631565284771023248631",
			decoded.1[1][1]
		);
		assert_eq!(
			"10284603410671614550643238877116026784009997646397200180055169244522533893335",
			decoded.1[1][0]
		);
		assert_eq!(
			"8376666972810749572085581968561346381911579868801081275529626269155085447161",
			decoded.2[0]
		);
		assert_eq!(
			"2463724514031046292864306191243943409912346551164607808423034641717054699949",
			decoded.2[1]
		);
	}

	#[test]
	fn test_input_hashing_encode_packed() {
		let requested_input = hex!(
			"0ab2afdc05c8b6ae1f2ab20874fb4159e25d5c1d4faa41aee232d6ab331332df0000000000747ffe"
		);
		let requested_input_hash = sha2_256(requested_input.as_slice());
		let stored_poseidon =
			U256::from("0ab2afdc05c8b6ae1f2ab20874fb4159e25d5c1d4faa41aee232d6ab331332df");
		let stored_slot = 7634942u64;
		let res = encode_packed(stored_poseidon, stored_slot);
		assert_eq!(requested_input_hash, sha2_256(res.as_slice()))
	}
}
