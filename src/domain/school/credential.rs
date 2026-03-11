use crate::domain::error::DomainError;

pub const SCHOOL_CREDENTIAL_VERSION_V1: u8 = 1;
pub const SCHOOL_CREDENTIAL_ALG_AES_256_GCM: &str = "AES-256-GCM";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolCredential {
    version: u8,
    algorithm: String,
    params_b64: String,
    salt_b64: String,
    nonce_b64: String,
    ciphertext_b64: String,
}

impl SchoolCredential {
    pub fn new_v1(
        params_b64: String,
        salt_b64: String,
        nonce_b64: String,
        ciphertext_b64: String,
    ) -> Result<Self, DomainError> {
        Self::new(
            SCHOOL_CREDENTIAL_VERSION_V1,
            SCHOOL_CREDENTIAL_ALG_AES_256_GCM.to_string(),
            params_b64,
            salt_b64,
            nonce_b64,
            ciphertext_b64,
        )
    }

    pub fn new(
        version: u8,
        algorithm: String,
        params_b64: String,
        salt_b64: String,
        nonce_b64: String,
        ciphertext_b64: String,
    ) -> Result<Self, DomainError> {
        if version != SCHOOL_CREDENTIAL_VERSION_V1 {
            return Err(DomainError::UnsupportedCredentialVersion { version });
        }
        if algorithm.trim().is_empty() {
            return Err(DomainError::InvalidCredentialAlgorithm);
        }
        if params_b64.trim().is_empty()
            || salt_b64.trim().is_empty()
            || nonce_b64.trim().is_empty()
            || ciphertext_b64.trim().is_empty()
        {
            return Err(DomainError::InvalidCredentialEnvelope);
        }

        Ok(Self {
            version,
            algorithm,
            params_b64,
            salt_b64,
            nonce_b64,
            ciphertext_b64,
        })
    }

    pub fn from_storage(storage: &str) -> Result<Self, DomainError> {
        let mut parts = storage.split(':');
        let version = parts
            .next()
            .ok_or(DomainError::InvalidCredentialEnvelope)?
            .parse::<u8>()
            .map_err(|_| DomainError::InvalidCredentialEnvelope)?;
        let algorithm = parts
            .next()
            .ok_or(DomainError::InvalidCredentialEnvelope)?
            .to_string();
        let params_b64 = parts
            .next()
            .ok_or(DomainError::InvalidCredentialEnvelope)?
            .to_string();
        let salt_b64 = parts
            .next()
            .ok_or(DomainError::InvalidCredentialEnvelope)?
            .to_string();
        let nonce_b64 = parts
            .next()
            .ok_or(DomainError::InvalidCredentialEnvelope)?
            .to_string();
        let ciphertext_b64 = parts
            .next()
            .ok_or(DomainError::InvalidCredentialEnvelope)?
            .to_string();

        if parts.next().is_some() {
            return Err(DomainError::InvalidCredentialEnvelope);
        }

        Self::new(
            version,
            algorithm,
            params_b64,
            salt_b64,
            nonce_b64,
            ciphertext_b64,
        )
    }

    pub fn to_storage(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.version,
            self.algorithm,
            self.params_b64,
            self.salt_b64,
            self.nonce_b64,
            self.ciphertext_b64,
        )
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    pub fn params_b64(&self) -> &str {
        &self.params_b64
    }

    pub fn salt_b64(&self) -> &str {
        &self.salt_b64
    }

    pub fn nonce_b64(&self) -> &str {
        &self.nonce_b64
    }

    pub fn ciphertext_b64(&self) -> &str {
        &self.ciphertext_b64
    }
}
