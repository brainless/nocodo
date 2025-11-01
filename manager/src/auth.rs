use crate::error::{AppError, AppResult};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// Hash a password using Argon2id with OWASP recommended parameters
pub fn hash_password(password: &str) -> AppResult<String> {
    use argon2::password_hash::rand_core::OsRng;
    let salt = SaltString::generate(&mut OsRng);

    // Use Argon2id with OWASP recommended parameters:
    // memory cost: 19456 KiB (19 MiB)
    // time cost (iterations): 2
    // parallelism: 1
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?
        .to_string();

    Ok(password_hash)
}

/// Verify a password against an Argon2id hash
pub fn verify_password(password: &str, password_hash: &str) -> AppResult<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|e| AppError::Internal(format!("Failed to parse password hash: {}", e)))?;

    let argon2 = Argon2::default();

    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// JWT claims for authentication tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,                     // Subject (user ID)
    pub username: String,                // Username for convenience
    pub exp: i64,                        // Expiration time (Unix timestamp)
    pub iat: i64,                        // Issued at (Unix timestamp)
    pub ssh_fingerprint: Option<String>, // Optional SSH key fingerprint used for login
}

impl Claims {
    /// Create new claims with default expiration (24 hours)
    pub fn new(user_id: i64, username: String, ssh_fingerprint: Option<String>) -> Self {
        let now = Utc::now().timestamp();
        let exp = now + (24 * 60 * 60); // 24 hours from now

        Self {
            sub: user_id.to_string(),
            username,
            exp,
            iat: now,
            ssh_fingerprint,
        }
    }

    /// Create claims with custom expiration duration (in seconds)
    #[allow(dead_code)]
    pub fn new_with_duration(
        user_id: i64,
        username: String,
        ssh_fingerprint: Option<String>,
        duration_seconds: i64,
    ) -> Self {
        let now = Utc::now().timestamp();
        let exp = now + duration_seconds;

        Self {
            sub: user_id.to_string(),
            username,
            exp,
            iat: now,
            ssh_fingerprint,
        }
    }
}

/// Generate a JWT token from claims
pub fn generate_token(claims: &Claims, secret: &str) -> AppResult<String> {
    let token = encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Failed to generate JWT token: {}", e)))?;

    Ok(token)
}

/// Validate and decode a JWT token
pub fn validate_token(token: &str, secret: &str) -> AppResult<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| AppError::Unauthorized(format!("Invalid token: {}", e)))?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123!";
        let hash = hash_password(password).unwrap();

        // Verify correct password
        assert!(verify_password(password, &hash).unwrap());

        // Verify incorrect password
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_jwt_token_generation_and_validation() {
        let secret = "test_secret_key_for_jwt";
        let user_id = 42;
        let username = "testuser".to_string();
        let fingerprint = Some("SHA256:test123".to_string());

        // Generate token
        let claims = Claims::new(user_id, username.clone(), fingerprint.clone());
        let token = generate_token(&claims, secret).unwrap();

        // Validate token
        let decoded_claims = validate_token(&token, secret).unwrap();

        assert_eq!(decoded_claims.sub, user_id.to_string());
        assert_eq!(decoded_claims.username, username);
        assert_eq!(decoded_claims.ssh_fingerprint, fingerprint);
    }

    #[test]
    fn test_jwt_token_with_wrong_secret() {
        let secret = "test_secret_key";
        let wrong_secret = "wrong_secret_key";

        let claims = Claims::new(1, "testuser".to_string(), None);
        let token = generate_token(&claims, secret).unwrap();

        // Should fail with wrong secret
        assert!(validate_token(&token, wrong_secret).is_err());
    }
}
