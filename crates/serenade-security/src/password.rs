//! Password hashing (Symfony `PasswordHasherInterface` analogue).

use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher as _, PasswordVerifier, SaltString};

use crate::SecurityError;

/// Hashes and verifies password strings.
pub trait PasswordHasher: Send + Sync {
    /// Returns a PHC-formatted hash for `plain`.
    ///
    /// # Errors
    ///
    /// Returns [`SecurityError::Password`] when hashing fails or `plain` is empty.
    fn hash(&self, plain: &str) -> Result<String, SecurityError>;

    /// Returns `true` when `plain` matches `hashed`.
    ///
    /// # Errors
    ///
    /// Returns [`SecurityError::Password`] when `hashed` is not a valid PHC string.
    fn verify(&self, hashed: &str, plain: &str) -> Result<bool, SecurityError>;
}

/// Argon2id hasher using the crate defaults (PHC string output).
#[derive(Clone, Copy, Debug, Default)]
pub struct Argon2idPasswordHasher;

impl Argon2idPasswordHasher {
    /// Default Argon2id parameters.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PasswordHasher for Argon2idPasswordHasher {
    fn hash(&self, plain: &str) -> Result<String, SecurityError> {
        if plain.is_empty() {
            return Err(SecurityError::Password {
                message: String::from("password must not be empty"),
            });
        }
        hash_password_with_salt(plain, &random_salt())
    }

    fn verify(&self, hashed: &str, plain: &str) -> Result<bool, SecurityError> {
        let parsed = PasswordHash::new(hashed).map_err(|error| SecurityError::Password {
            message: error.to_string(),
        })?;
        Ok(Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok())
    }
}

fn random_salt() -> SaltString {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("OS RNG must provide salt bytes");
    SaltString::encode_b64(&bytes).expect("16 random bytes always encode as salt")
}

fn hash_password_with_salt(plain: &str, salt: &SaltString) -> Result<String, SecurityError> {
    Argon2::default()
        .hash_password(plain.as_bytes(), salt)
        .map(|hash| hash.to_string())
        .map_err(|error| SecurityError::Password {
            message: error.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use argon2::password_hash::SaltString;

    use super::{Argon2idPasswordHasher, PasswordHasher, hash_password_with_salt};
    use crate::SecurityError;

    #[test]
    fn hash_verify_roundtrip_and_reject_wrong() {
        let hasher = Argon2idPasswordHasher::new();
        let encoded = hasher.hash("correct horse").expect("hash");
        assert!(encoded.starts_with("$argon2id$"));
        assert!(hasher.verify(&encoded, "correct horse").expect("ok"));
        assert!(!hasher.verify(&encoded, "wrong").expect("mismatch"));
        assert_ne!(hasher.hash("correct horse").expect("again"), encoded);
    }

    #[test]
    fn empty_plain_rejected_and_bad_hash_errors() {
        let hasher = Argon2idPasswordHasher;
        assert!(matches!(
            hasher.hash(""),
            Err(SecurityError::Password { .. })
        ));
        assert!(matches!(
            hasher.verify("not-a-phc", "x"),
            Err(SecurityError::Password { .. })
        ));
    }

    #[test]
    fn hash_maps_argon2_errors_for_short_salt() {
        // SaltString allows 4+ bytes; Argon2 rejects salts shorter than 8 bytes.
        let salt = SaltString::from_b64("AAAAAAAA").expect("6-byte salt");
        assert!(matches!(
            hash_password_with_salt("secret", &salt),
            Err(SecurityError::Password { .. })
        ));
    }
}
