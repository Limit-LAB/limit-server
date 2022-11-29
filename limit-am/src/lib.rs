use std::error::Error;

use limit_deps::*;

use aes::{
    cipher::{BlockDecrypt, BlockEncrypt, KeyInit},
    Aes256,
};
use anyhow::Context;
use elliptic_curve::{ecdh::SharedSecret, generic_array::GenericArray, sec1::ToEncodedPoint};
use p256::{NistP256, PublicKey, SecretKey};

pub fn create_random_secret() -> Result<(String, String), Box<dyn Error>> {
    let secret_key = SecretKey::random(&mut rand::rngs::OsRng);
    let der = secret_key.to_sec1_der().map_err(|err| err.to_string())?;
    let binding = secret_key.public_key().to_encoded_point(false);
    let derp = binding.as_bytes();
    let der_base64 = base64::encode(&der);
    let derp_base64 = base64::encode(derp);
    Ok((der_base64, derp_base64))
}

pub fn decode_secret(secret: &str) -> Result<SecretKey, Box<dyn Error>> {
    let der = base64::decode(secret).map_err(|err| err.to_string())?;
    let secret_key = SecretKey::from_sec1_der(&der).map_err(|err| err.to_string())?;
    Ok(secret_key)
}

pub fn decode_public(public: &str) -> Result<PublicKey, Box<dyn Error>> {
    let der = base64::decode(public).map_err(|err| err.to_string())?;
    let public_key = PublicKey::from_sec1_bytes(&der).map_err(|err| err.to_string())?;
    Ok(public_key)
}

pub fn key_exchange(privkey1: SecretKey, pubkey2: PublicKey) -> String {
    let shared_secret =
        elliptic_curve::ecdh::diffie_hellman(privkey1.to_nonzero_scalar(), pubkey2.as_affine());
    let encoded = base64::encode(shared_secret.raw_secret_bytes());
    encoded
}

pub fn decode_shared_key(encoded: String) -> SharedSecret<NistP256> {
    SharedSecret::from(*GenericArray::from_slice(
        base64::decode(encoded).unwrap_or_default().as_slice(),
    ))
}

pub fn aes256_encrypt_string(key: &str, plaintext: &str) -> Result<String, Box<dyn Error>> {
    let binding = base64::decode(key)?;
    let key = binding.as_slice();
    let cipher = Aes256::new_from_slice(key).map_err(|err| err.to_string())?;
    let padding = 16 - (plaintext.as_bytes().len() % 16);
    let padded_text_bytes = [plaintext.as_bytes(), &[padding as u8; 16][..padding]].concat();
    debug_assert!(padded_text_bytes.len() % 16 == 0);
    let res = padded_text_bytes
        .chunks(16)
        .flat_map(|chunk| {
            let mut block = GenericArray::clone_from_slice(chunk);
            cipher.encrypt_block(&mut block);
            block
        })
        .collect::<Vec<u8>>();
    Ok(base64::encode(res))
}

pub fn aes256_decrypt_string(key: &str, ciphertext: &str) -> Result<String, Box<dyn Error>> {
    let binding = base64::decode(key)?;
    let key = binding.as_slice();
    let cipher = Aes256::new_from_slice(key).map_err(|err| err.to_string())?;
    let plaintext = base64::decode(ciphertext)?;
    debug_assert!(plaintext.len() % 16 == 0);
    let plaintext = plaintext
        .chunks(16)
        .flat_map(|chunk| {
            let mut block = GenericArray::clone_from_slice(chunk);
            cipher.decrypt_block(&mut block);
            block
        })
        .collect::<Vec<u8>>();
    let padder = plaintext.len().checked_sub(1).context("arith error")?;
    let padding = plaintext[padder];
    let plaintext = &plaintext[..plaintext.len() - padding as usize];
    Ok(String::from_utf8(plaintext.to_vec())?)
}

#[test]
fn test_key_exchange_encode_decode() {
    let (user1_secret, user1_public) = create_random_secret().unwrap();
    let (user2_secret, user2_public) = create_random_secret().unwrap();
    let user1_secret_decoded = decode_secret(&user1_secret).unwrap();
    let user2_secret_decoded = decode_secret(&user2_secret).unwrap();
    let user1_pubkey = decode_public(&user1_public).unwrap();
    let user2_pubkey = decode_public(&user2_public).unwrap();

    let user1_to_2_shared_secret = key_exchange(user1_secret_decoded, user2_pubkey);
    let user2_to_1_shared_secret = key_exchange(user2_secret_decoded, user1_pubkey);
    assert_eq!(user1_to_2_shared_secret, user2_to_1_shared_secret);
    println!("Shared secret: {}", user1_to_2_shared_secret);

    println!();
    // user 1 send message to user 2
    let plaintext = "hello user 2 how do you do";
    println!("plaintext: {}", plaintext);
    let ciphertext1_to_2 = aes256_encrypt_string(&user1_to_2_shared_secret, plaintext).unwrap();
    println!("ciphertext: {}", ciphertext1_to_2);
    let decoded1_to_2 =
        aes256_decrypt_string(&user2_to_1_shared_secret, &ciphertext1_to_2).unwrap();
    println!("decoded from user2: {}", decoded1_to_2);
    assert_eq!(plaintext, decoded1_to_2);

    println!();
    // user 2 send message to user 1
    let plaintext = "hi user 1, nice to meet you";
    println!("plaintext: {}", plaintext);
    let ciphertext2_to_1 = aes256_encrypt_string(&user2_to_1_shared_secret, plaintext).unwrap();
    println!("ciphertext: {}", ciphertext2_to_1);
    let decoded2_to_1 =
        aes256_decrypt_string(&user1_to_2_shared_secret, &ciphertext2_to_1).unwrap();
    println!("decoded from user1: {}", decoded2_to_1);
    assert_eq!(plaintext, decoded2_to_1);
}
