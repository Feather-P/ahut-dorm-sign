use crate::utils::hash::encode_md5;
use crate::utils::headers::insert_header_str;
use crate::utils::url::to_sign_path;
use crate::{constants::auth::APP_AUTHORIZATION, utils::hash::encode_base64};
use reqwest::header::{AUTHORIZATION as HEADER_AUTHORIZATION, HeaderMap, InvalidHeaderValue};
use chrono::Utc;

/// 生成 `FlySource-Auth` 请求头，格式为 `bearer {token}`
pub fn generate_flysource_auth(token: &str) -> String {
    format!("bearer {}", token)
}

/// 生成当前毫秒级时间戳
fn current_timestamp_millis() -> i64 {
    Utc::now().timestamp_millis()
}

/// 生成 `FlySource-sign`
///
/// 实现算法：
/// 1) hash1 = MD5(timestamp + token)
/// 2) hash2 = MD5(url_path + "?sign=" + hash1)
/// 3) sign  = hash2 + "1." + Base64(timestamp)
pub fn generate_flysource_sign(url: &str, token: &str) -> String {
    let timestamp = current_timestamp_millis().to_string();
    let hash1 = encode_md5(&format!("{}{}", timestamp, token));
    let hash2 = encode_md5(&format!("{}{}", to_sign_path(url), hash1));
    format!("{}1.{}", hash2, encode_base64(timestamp.as_bytes()))
}

/// 构造业务接口（含 FlySource 签名）请求头
pub fn build_app_signed_headers(url: &str, token: &str) -> Result<HeaderMap, InvalidHeaderValue> {
    let mut headers = HeaderMap::with_capacity(3);

    insert_header_str(&mut headers, HEADER_AUTHORIZATION, APP_AUTHORIZATION)?;
    insert_header_str(
        &mut headers,
        "flysource-auth",
        &generate_flysource_auth(token),
    )?;
    insert_header_str(
        &mut headers,
        "flysource-sign",
        &generate_flysource_sign(url, token),
    )?;

    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_generate_flysource_auth() {
        assert_eq!(generate_flysource_auth("token-123"), "bearer token-123");
    }

    #[test]
    fn should_strip_query_when_building_sign_base_url() {
        assert_eq!(
            to_sign_path("https://example.com/api/auth/login?x=1&y=2"),
            "/api/auth/login?sign="
        );
    }

    #[test]
    fn should_fallback_when_url_parse_failed() {
        assert_eq!(
            to_sign_path("/api/auth/login?x=1&y=2"),
            "/api/auth/login?sign="
        );
    }
}
