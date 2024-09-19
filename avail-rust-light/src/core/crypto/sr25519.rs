use std::fmt::Display;

use super::{derive_junction::DeriveJunction, secret_uri::SecretUri, AccountId};
use parity_scale_codec::Encode;
use schnorrkel::{
	derive::{ChainCode, Derivation},
	ExpansionMode, MiniSecretKey,
};

const SIGNING_CTX: &[u8] = b"substrate";

/// A signature generated by [`Keypair::sign()`]. These bytes are equivalent
/// to a Substrate `MultiSignature::sr25519(bytes)`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);
impl Encode for Signature {
	fn size_hint(&self) -> usize {
		self.0.len()
	}

	fn encode_to<T: parity_scale_codec::Output + ?Sized>(&self, dest: &mut T) {
		self.0.encode_to(dest);
	}
}

/// The public key for an [`Keypair`] key pair. This is equivalent to a
/// Substrate `AccountId32`.
pub struct PublicKey(pub [u8; 32]);

impl PublicKey {
	pub fn to_account_id(self) -> AccountId {
		AccountId(self.0)
	}
}

impl Encode for PublicKey {
	fn size_hint(&self) -> usize {
		self.0.len()
	}

	fn encode_to<T: parity_scale_codec::Output + ?Sized>(&self, dest: &mut T) {
		self.0.encode_to(dest);
	}
}

const SECRET_KEY_LENGTH: usize = schnorrkel::keys::MINI_SECRET_KEY_LENGTH;
/// Seed bytes used to generate a key pair.
pub type SecretKeyBytes = [u8; SECRET_KEY_LENGTH];

#[derive(Debug, Clone)]
pub struct Keypair(schnorrkel::Keypair);

impl Keypair {
	pub fn sign(&self, message: &[u8]) -> Signature {
		let context = schnorrkel::signing_context(SIGNING_CTX);
		let signature = self.0.sign(context.bytes(message));
		Signature(signature.to_bytes())
	}

	pub fn verify(sig: &Signature, message: &[u8], public_key: PublicKey) -> bool {
		let Ok(signature) = schnorrkel::Signature::from_bytes(&sig.0) else {
			return false;
		};
		let Ok(public) = schnorrkel::PublicKey::from_bytes(&public_key.0) else {
			return false;
		};
		public
			.verify_simple(SIGNING_CTX, message.as_ref(), &signature)
			.is_ok()
	}

	/// Create am sr25519 keypair from a [`SecretUri`]. See the [`SecretUri`] docs for more.
	///
	/// # Example
	///
	/// ```rust
	/// use subxt_signer::{ SecretUri, sr25519::Keypair };
	/// use std::str::FromStr;
	///
	/// let uri = SecretUri::from_str("//Alice").unwrap();
	/// let keypair = Keypair::from_uri(&uri).unwrap();
	///
	/// keypair.sign(b"Hello world!");
	/// ```
	pub fn from_uri(uri: &SecretUri) -> Result<Self, Error> {
		use hex::FromHex;
		use secrecy::ExposeSecret;
		use std::str::FromStr;

		let SecretUri {
			junctions,
			phrase,
			password,
		} = uri;

		// If the phrase is hex, convert bytes directly into a seed, ignoring password.
		// Else, parse the phrase string taking the password into account. This is
		// the same approach taken in sp_core::crypto::Pair::from_string_with_seed.
		let key = if let Some(hex_str) = phrase.expose_secret().strip_prefix("0x") {
			let seed = SecretKeyBytes::from_hex(hex_str)?;
			Self::from_secret_key(seed)?
		} else {
			let phrase = bip39::Mnemonic::from_str(phrase.expose_secret().as_str())?;
			let pass_str = password.as_ref().map(|p| p.expose_secret().as_str());
			Self::from_phrase(&phrase, pass_str)?
		};

		// Now, use any "junctions" to derive a new key from this root key.
		Ok(key.derive(junctions.iter().copied()))
	}

	/// Create am sr25519 keypair from a BIP-39 mnemonic phrase and optional password.
	///
	/// # Example
	///
	/// ```rust
	/// use subxt_signer::{ bip39::Mnemonic, sr25519::Keypair };
	///
	/// let phrase = "bottom drive obey lake curtain smoke basket hold race lonely fit walk";
	/// let mnemonic = Mnemonic::parse(phrase).unwrap();
	/// let keypair = Keypair::from_phrase(&mnemonic, None).unwrap();
	///
	/// keypair.sign(b"Hello world!");
	/// ```
	pub fn from_phrase(mnemonic: &bip39::Mnemonic, password: Option<&str>) -> Result<Self, Error> {
		let (arr, len) = mnemonic.to_entropy_array();
		let big_seed =
			seed_from_entropy(&arr[0..len], password.unwrap_or("")).ok_or(Error::InvalidSeed)?;

		let seed: SecretKeyBytes = big_seed[..SECRET_KEY_LENGTH]
			.try_into()
			.expect("should be valid Seed");

		Self::from_secret_key(seed)
	}

	/// Turn a 32 byte secret key into a keypair.
	///
	/// # Warning
	///
	/// This will only be secure if the seed is secure!
	pub fn from_secret_key(secret_key_bytes: SecretKeyBytes) -> Result<Self, Error> {
		let keypair = MiniSecretKey::from_bytes(&secret_key_bytes)
			.map_err(|_| Error::InvalidSeed)?
			.expand_to_keypair(ExpansionMode::Ed25519);

		Ok(Keypair(keypair))
	}

	/// Derive a child key from this one given a series of junctions.
	///
	/// # Example
	///
	/// ```rust
	/// use subxt_signer::{ bip39::Mnemonic, sr25519::Keypair, DeriveJunction };
	///
	/// let phrase = "bottom drive obey lake curtain smoke basket hold race lonely fit walk";
	/// let mnemonic = Mnemonic::parse(phrase).unwrap();
	/// let keypair = Keypair::from_phrase(&mnemonic, None).unwrap();
	///
	/// // Equivalent to the URI path '//Alice/stash':
	/// let new_keypair = keypair.derive([
	///     DeriveJunction::hard("Alice"),
	///     DeriveJunction::soft("stash")
	/// ]);
	/// ```
	pub fn derive<Js: IntoIterator<Item = DeriveJunction>>(&self, junctions: Js) -> Self {
		let init = self.0.secret.clone();
		let result = junctions.into_iter().fold(init, |acc, j| match j {
			DeriveJunction::Soft(cc) => acc.derived_key_simple(ChainCode(cc), []).0,
			DeriveJunction::Hard(cc) => {
				let seed = acc.hard_derive_mini_secret_key(Some(ChainCode(cc)), b"").0;
				seed.expand(ExpansionMode::Ed25519)
			},
		});
		Self(result.into())
	}

	fn account_id(&self) -> PublicKey {
		self.public_key().into()
	}

	fn address(&self) -> PublicKey {
		self.public_key().into()
	}

	/// Obtain the [`PublicKey`] part of this key pair, which can be used in calls to [`verify()`].
	/// or otherwise converted into an address. The public key bytes are equivalent to a Substrate
	/// `AccountId32`.
	pub fn public_key(&self) -> PublicKey {
		PublicKey(self.0.public.to_bytes())
	}
}

/// An error handed back if creating a keypair fails.
#[derive(Debug)]
pub enum Error {
	/// Invalid seed.
	InvalidSeed,
	/// Invalid phrase.
	Phrase(bip39::Error),
	/// Invalid hex.
	Hex(hex::FromHexError),
}

impl From<bip39::Error> for Error {
	fn from(value: bip39::Error) -> Self {
		Error::Phrase(value)
	}
}

impl From<hex::FromHexError> for Error {
	fn from(value: hex::FromHexError) -> Self {
		Error::Hex(value)
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Error::InvalidSeed => write!(f, "Invalid seed (was it the wrong length?)"),
			Error::Phrase(e) => write!(f, "Cannot parse phrase: {e}"),
			Error::Hex(e) => write!(f, "Cannot parse hex string: {e}"),
		}
	}
}

/// This is taken from `substrate-bip39` so that we can keep dependencies in line, and
/// is the same logic that sp-core uses to go from mnemonic entropy to seed. Returns
/// `None` if invalid length.
#[allow(dead_code)]
pub fn seed_from_entropy(entropy: &[u8], password: &str) -> Option<[u8; 64]> {
	use hmac::Hmac;
	use pbkdf2::pbkdf2;
	use secrecy::Zeroize;
	use sha2::Sha512;

	if entropy.len() < 16 || entropy.len() > 32 || entropy.len() % 4 != 0 {
		return None;
	}

	let mut salt = String::with_capacity(8 + password.len());
	salt.push_str("mnemonic");
	salt.push_str(password);

	let mut seed = [0u8; 64];

	pbkdf2::<Hmac<Sha512>>(entropy, salt.as_bytes(), 2048, &mut seed).ok()?;

	salt.zeroize();

	Some(seed)
}
