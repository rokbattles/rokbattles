use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngExt;
use sha2::Digest;

pub(super) fn generate_code_verifier() -> String {
    random_token(64)
}

pub(super) fn derive_code_challenge(verifier: &str) -> String {
    let digest = sha2::Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

pub(super) fn generate_state() -> String {
    random_token(16)
}

pub(super) fn generate_session_id() -> String {
    random_token(48)
}

fn random_token(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    let mut rng = rand::rng();
    rng.fill(bytes.as_mut_slice());
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_random_tokens() {
        let state_a = generate_state();
        let state_b = generate_state();

        assert!(!state_a.is_empty());
        assert_ne!(state_a, state_b);
    }

    #[test]
    fn derives_code_challenge() {
        let verifier = "test-verifier";
        let challenge = derive_code_challenge(verifier);
        assert_eq!(challenge, "JBbiqONGWPaAmwXk_8bT6UnlPfrn65D32eZlJS-zGG0");
    }
}
