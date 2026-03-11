use chrono::{DateTime, Utc};

pub trait SchoolSignGenerator {
    fn sign(&self, access_token: &str, url: &str, time_now: DateTime<Utc>) -> String;
    fn auth(&self, access_token: &str) -> String;
}
