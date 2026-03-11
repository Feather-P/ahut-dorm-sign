use crate::domain::{error::DomainError, school::credential::SchoolCredential};

pub trait SchoolSidePasswdHasher {
    fn hash(&self, plain_password: &str) -> String;
}

pub trait SchoolCredentialProtector {
    fn encrypt(&self, school_origin_credential: &str) -> SchoolCredential;
    fn decrypt(&self, school_credential: &SchoolCredential) -> Result<String, DomainError>;
}
