use rand::Rng;

/// Generates a base58-encoded random token with `byte_len` bytes of CSPRNG
/// entropy. Used for auth nonces (16 bytes = 128 bits, matching the QR
/// `jti` entropy requirement in spec §8.2) and session IDs (32 bytes).
pub fn generate_token(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    rand::rng().fill_bytes(&mut bytes);
    bs58::encode(bytes).into_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique_and_correctly_sized() {
        let a = generate_token(16);
        let b = generate_token(16);
        assert_ne!(a, b);
        // Base58-decoded length must match the requested entropy exactly.
        assert_eq!(bs58::decode(&a).into_vec().unwrap().len(), 16);
    }
}
