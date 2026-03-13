use chrono::{DateTime, Utc};

/// 学校的业务端点签名机制接口
pub trait SchoolSignGenerator {
    fn sign(&self, access_token: &str, url: &str, time_now: DateTime<Utc>) -> String;
    fn auth(&self, access_token: &str) -> String;
}
