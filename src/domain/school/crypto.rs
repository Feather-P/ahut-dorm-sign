use chrono::{DateTime, Utc};

pub trait SchoolPasswordHasher {
    fn hash(&self, plain_password: &str) -> String;
    fn verify(&self, plain_password: &str, hashed_password_: &str) -> bool;
}

pub trait SchoolSignGenerator {
    fn sign(&self, access_token: &str, url: &str, time_now: DateTime<Utc>) -> String;
    fn auth(&self, access_token: &str) -> String;
}
