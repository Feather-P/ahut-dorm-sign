use std::env;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;

use crate::domain::{
    error::DomainError,
    school::{
        credential::SchoolCredential,
        crypto::SchoolCredentialProtector,
        sign::SchoolSignGenerator,
    },
};

const PBKDF2_ITER: u32 = 120_000;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

pub struct AesGcmSchoolCredentialProtector {
    master_key: String,
}

impl AesGcmSchoolCredentialProtector {
    pub fn from_env() -> Result<Self, DomainError> {
        let _ = dotenv();
        let master_key = env::var("SCHOOL_CREDENTIAL_MASTER_KEY")
            .map_err(|_| DomainError::InvalidCredentialEnvelope)?;
        if master_key.trim().is_empty() {
            return Err(DomainError::InvalidCredentialEnvelope);
        }
        Ok(Self { master_key })
    }

    fn derive_key(&self, salt: &[u8], iter: u32) -> [u8; KEY_LEN] {
        let mut key = [0u8; KEY_LEN];
        pbkdf2_hmac::<Sha256>(self.master_key.as_bytes(), salt, iter, &mut key);
        key
    }
}

impl SchoolCredentialProtector for AesGcmSchoolCredentialProtector {
    fn encrypt(&self, school_origin_credential: &str) -> SchoolCredential {
        let mut salt = [0u8; SALT_LEN];
        let mut nonce = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut salt);
        rand::thread_rng().fill_bytes(&mut nonce);

        let key = self.derive_key(&salt, PBKDF2_ITER);
        let cipher = Aes256Gcm::new_from_slice(&key).expect("invalid aes key length");
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), school_origin_credential.as_bytes())
            .expect("encrypt failed");

        let params = serde_json::json!({
            "kdf": "pbkdf2-sha256",
            "iter": PBKDF2_ITER
        });

        SchoolCredential::new_v1(
            STANDARD.encode(params.to_string()),
            STANDARD.encode(salt),
            STANDARD.encode(nonce),
            STANDARD.encode(ciphertext),
        )
        .expect("school credential envelope invalid")
    }

    fn decrypt(&self, school_credential: &SchoolCredential) -> Result<String, DomainError> {
        let params_raw = STANDARD
            .decode(school_credential.params_b64())
            .map_err(|_| DomainError::InvalidCredentialEnvelope)?;
        let params: serde_json::Value =
            serde_json::from_slice(&params_raw).map_err(|_| DomainError::InvalidCredentialEnvelope)?;

        let iter = params
            .get("iter")
            .and_then(|v| v.as_u64())
            .ok_or(DomainError::InvalidCredentialEnvelope)? as u32;
        let kdf = params
            .get("kdf")
            .and_then(|v| v.as_str())
            .ok_or(DomainError::InvalidCredentialEnvelope)?;
        if kdf != "pbkdf2-sha256" {
            return Err(DomainError::InvalidCredentialAlgorithm);
        }

        let salt = STANDARD
            .decode(school_credential.salt_b64())
            .map_err(|_| DomainError::InvalidCredentialEnvelope)?;
        let nonce = STANDARD
            .decode(school_credential.nonce_b64())
            .map_err(|_| DomainError::InvalidCredentialEnvelope)?;
        let ciphertext = STANDARD
            .decode(school_credential.ciphertext_b64())
            .map_err(|_| DomainError::InvalidCredentialEnvelope)?;

        if nonce.len() != NONCE_LEN {
            return Err(DomainError::InvalidCredentialEnvelope);
        }

        let key = self.derive_key(&salt, iter);
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| DomainError::CredentialDecryptFailed)?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| DomainError::CredentialDecryptFailed)?;

        String::from_utf8(plaintext).map_err(|_| DomainError::CredentialDecryptFailed)
    }
}

#[derive(Default)]
pub struct FlySourceSigner;

impl SchoolSignGenerator for FlySourceSigner {
    /// 生成学校请求头业务接口需要的Flysource-sign请求头
    fn sign(&self, access_token: &str, url: &str, time_now: DateTime<Utc>) -> String {
        let ts = time_now.timestamp_millis();
        let path = url.split('?').next().unwrap_or(url);
        let first = format!("{ts}{access_token}");
        let first_hash = format!("{:x}", md5::compute(first));
        let second = format!("{path}?sign={first_hash}");
        let second_hash = format!("{:x}", md5::compute(second));
        let encoded_ts = STANDARD.encode(ts.to_string());
        format!("{second_hash}1.{encoded_ts}")
    }

    /// 生成学校请求头业务接口需要的Flysource-auth请求
    fn auth(&self, access_token: &str) -> String {
        format!("bearer {access_token}")
    }
}

