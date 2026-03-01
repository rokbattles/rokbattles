use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngExt;
use sha2::Digest;

pub(super) fn build_avatar_url(id: &str, avatar: Option<&str>) -> Option<String> {
    let avatar = avatar?;
    let extension = if avatar.starts_with("a_") {
        "gif"
    } else {
        "png"
    };
    Some(format!(
        "https://cdn.discordapp.com/avatars/{id}/{avatar}.{extension}?size=256"
    ))
}

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

    #[test]
    fn builds_avatar_url_when_avatar_is_present() {
        let animated = build_avatar_url("123", Some("a_hash")).expect("url");
        let static_url = build_avatar_url("456", Some("hash")).expect("url");

        assert_eq!(
            animated,
            "https://cdn.discordapp.com/avatars/123/a_hash.gif?size=256"
        );
        assert_eq!(
            static_url,
            "https://cdn.discordapp.com/avatars/456/hash.png?size=256"
        );
    }

    #[test]
    fn returns_none_avatar_url_when_avatar_is_missing() {
        assert_eq!(build_avatar_url("123", None), None);
    }
}
