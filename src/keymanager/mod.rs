use rand::RngCore;
use rand::rngs::OsRng;
use ed25519_dalek::{SigningKey, SECRET_KEY_LENGTH};
use bip39::{Mnemonic, Language};
use sha2::{Sha256, Digest as ShaDigest};
use hex;

/// Generates a new key pair with a prefix and returns the public key, private key, mnemonic phrase, and address.
pub fn generate_key_pair_with_prefix(prefix: &str) -> (String, String, String, String) {
    // Generate a random 128-bit (16 bytes) entropy
    let mut entropy = [0u8; 32];
    OsRng.fill_bytes(&mut entropy);

    // Create a mnemonic from the entropy
    let mnemonic = Mnemonic::from_entropy(&entropy).unwrap();

    // Derive a seed from the mnemonic
    let seed = mnemonic.to_seed("");

    // Generate a signing key using the seed
    let signing_key = SigningKey::from_bytes(
        &seed[0..SECRET_KEY_LENGTH].try_into().expect("slice with incorrect length")
    );

    // Get the verifying key from the signing key, and encode it in hexadecimal format
    let public_key = hex::encode(signing_key.verifying_key().as_bytes());

    // Get the bytes of the signing key, and encode them in hexadecimal format
    let private_key = hex::encode(signing_key.to_bytes());

    // Generate the address using the prefix and public key
    let address = format!("{}{}", prefix.to_lowercase(), generate_address(&public_key));

    // Return the public key, private key, mnemonic phrase, and address
    (public_key, private_key, mnemonic.to_string(), address)
}

/// Generates an address from the public key
pub fn generate_address(public_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..20])
}

/// Retrieves a key pair and address from a provided mnemonic phrase
pub fn generate_key_pair_from_mnemonic(mnemonic_phrase: &str, prefix: &str) -> (String, String, String) {
    // Parse the mnemonic from the provided phrase using the appropriate language
    let mnemonic = Mnemonic::parse_in(Language::English, mnemonic_phrase).expect("Invalid mnemonic phrase");

    // Derive a seed from the mnemonic
    let seed = mnemonic.to_seed("");

    // Generate a signing key using the seed
    let signing_key = SigningKey::from_bytes(
        &seed[0..SECRET_KEY_LENGTH].try_into().expect("slice with incorrect length")
    );

    // Get the verifying key from the signing key, and encode it in hexadecimal format
    let public_key = hex::encode(signing_key.verifying_key().as_bytes());

    // Get the bytes of the signing key, and encode them in hexadecimal format
    let private_key = hex::encode(signing_key.to_bytes());

    // Generate the address using the prefix and public key
    let address = format!("{}{}", prefix.to_lowercase(), generate_address(&public_key));

    // Return the public key, private key, and the derived address
    (public_key, private_key, address)
}
