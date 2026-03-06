use crate::utils::hash::encode_md5;
use crate::utils::headers::insert_header_str;
use crate::{constants::auth::APP_AUTHORIZATION, utils::hash::encode_base64};
use reqwest::header::{AUTHORIZATION as HEADER_AUTHORIZATION, HeaderMap};
use reqwest::Url;

use chrono::Utc;

/// 生成 `FlySource-Auth` 请求头，格式为 `bearer {token}`
pub fn generate_flysource_auth(token: &str) -> String {
    format!("bearer {}", token)
}

/// 生成当前毫秒级时间戳
fn current_timestamp_millis() -> i64 {
    Utc::now().timestamp_millis()
}

/// 获取用于签名计算的 URL 基础串（去掉 query 后追加 `?sign=`）
fn get_sign_base_url(url: &str) -> String {
    if let Ok(parsed) = Url::parse(url) {
        return format!("{}?sign=", parsed.path());
    }

    // 如果失败：按原始字符串去掉 query，兼容相对路径或非标准 URL 输入
    let path_without_query = url.split('?').next().unwrap_or(url);
    format!("{}?sign=", path_without_query)
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
    let hash2 = encode_md5(&format!("{}{}", get_sign_base_url(url), hash1));
    format!("{}1.{}", hash2, encode_base64(timestamp.as_bytes()))
}

/// 构造业务接口（含 FlySource 签名）请求头
pub fn build_app_signed_headers(url: &str, token: &str) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::with_capacity(3);

    insert_header_str(&mut headers, HEADER_AUTHORIZATION, APP_AUTHORIZATION)
        .map_err(|e| e.to_string())?;
    insert_header_str(&mut headers, "flysource-auth", &generate_flysource_auth(token))
        .map_err(|e| e.to_string())?;
    insert_header_str(&mut headers, "flysource-sign", &generate_flysource_sign(url, token))
        .map_err(|e| e.to_string())?;

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
            get_sign_base_url("https://example.com/api/auth/login?x=1&y=2"),
            "/api/auth/login?sign="
        );
    }

    #[test]
    fn should_fallback_when_url_parse_failed() {
        assert_eq!(
            get_sign_base_url("/api/auth/login?x=1&y=2"),
            "/api/auth/login?sign="
        );
    }
}
