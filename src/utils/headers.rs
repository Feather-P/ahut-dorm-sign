use reqwest::header::{HeaderMap, HeaderValue, IntoHeaderName, InvalidHeaderValue};

/// 向 `HeaderMap` 插入字符串值请求头。
///
/// 将 `HeaderValue::from_str` 的重复样板抽取为统一方法。
pub fn insert_header_str<K>(
    headers: &mut HeaderMap,
    key: K,
    value: &str,
) -> Result<(), InvalidHeaderValue>
where
    K: IntoHeaderName,
{
    headers.insert(key, HeaderValue::from_str(value)?);
    Ok(())
}
