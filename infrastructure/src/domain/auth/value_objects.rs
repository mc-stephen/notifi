//! Value objects and validation rules.

use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::domain::auth::errors::AuthError;

/// Validated, normalized (lowercased) email address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    /// Structural validation only (deliverability is unknowable).
    pub fn parse(raw: &str) -> Result<Self, AuthError> {
        let value = raw.trim().to_lowercase();

        let invalid = || AuthError::Validation("email address is not valid".to_string());

        if value.len() > 254 || value.contains(char::is_whitespace) {
            return Err(invalid());
        }
        let (local, domain) = value.split_once('@').ok_or_else(invalid)?;
        if local.is_empty() || local.len() > 64 {
            return Err(invalid());
        }
        // require a dotted domain — pragmatic, blocks the obvious garbage
        if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
            return Err(invalid());
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Enforces the dashboard password policy: min 8 chars with uppercase,
/// lowercase, number and special character (`lib/auth-types.ts` contract).
pub fn validate_password(password: &str) -> Result<(), AuthError> {
    let mut upper = false;
    let mut lower = false;
    let mut digit = false;
    let mut special = false;

    for ch in password.chars() {
        if ch.is_ascii_uppercase() {
            upper = true;
        } else if ch.is_ascii_lowercase() {
            lower = true;
        } else if ch.is_ascii_digit() {
            digit = true;
        } else if ch.is_ascii_punctuation() {
            special = true;
        }
    }

    let ok = password.len() >= 8 && upper && lower && digit && special;
    if ok {
        Ok(())
    } else {
        Err(AuthError::Validation(
            "password must be at least 8 characters and include an uppercase letter, \
             a lowercase letter, a number, and a special character"
                .to_string(),
        ))
    }
}

/// SHA-256 hex digest of a raw token/cookie value. Raw secrets are never
/// stored; only this hash is persisted and looked up.
pub fn hash_token(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    hex::encode(digest)
}

/// Generates a fresh random token: returns `(raw, hash)` — the raw value is
/// shown/emailed exactly once, the hash goes into storage.
pub fn new_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let raw = hex::encode(bytes);
    let hash = hash_token(&raw);
    (raw, hash)
}

/// URL-safe base64 without padding (RFC 7636 §Appendix A shape), used for
/// PKCE S256 `code_challenge` values.
pub fn urlsafe_b64(bytes: &[u8]) -> String {
    const CHARS: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b1 = chunk[0] as u32;
        let b2 = *chunk.get(1).unwrap_or(&0) as u32;
        let b3 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b1 << 16) | (b2 << 8) | b3;
        out.push(CHARS[(n >> 18) as usize & 63] as char);
        out.push(CHARS[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(CHARS[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(CHARS[n as usize & 63] as char);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_accepts_normal_addresses_and_lowercases() {
        let email = Email::parse("  Jane.Doe@Example.COM ").unwrap();
        assert_eq!(email.as_str(), "jane.doe@example.com");
    }

    #[test]
    fn email_rejects_garbage() {
        for bad in ["nope", "a@b", "a b@c.com", "@x.com", "a@.com", "a@com.", ""] {
            assert!(Email::parse(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn password_policy_matches_the_contract() {
        assert!(validate_password("Abcdef1!").is_ok());
        assert!(validate_password("abcdef1!").is_err()); // no uppercase
        assert!(validate_password("ABCDEF1!").is_err()); // no lowercase
        assert!(validate_password("Abcdefgh!").is_err()); // no digit
        assert!(validate_password("Abcdefg1").is_err()); // no special
        assert!(validate_password("Ab1!").is_err()); // too short
    }

    #[test]
    fn token_raw_and_hash_differ_and_are_stable() {
        let (raw, hash) = new_token();
        assert_ne!(raw, hash);
        assert_eq!(hash_token(&raw), hash);
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn urlsafe_b64_matches_known_vectors() {
        // RFC 4648 test vectors, padding stripped, +/ swapped for -_
        assert_eq!(urlsafe_b64(b""), "");
        assert_eq!(urlsafe_b64(b"f"), "Zg");
        assert_eq!(urlsafe_b64(b"fo"), "Zm8");
        assert_eq!(urlsafe_b64(b"foo"), "Zm9v");
        assert_eq!(urlsafe_b64(b"foobar"), "Zm9vYmFy");
    }
}
