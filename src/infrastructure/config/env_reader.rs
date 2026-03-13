use std::{env, str::FromStr};

use anyhow::{Context, Result, anyhow};

pub fn required_env(key: &str) -> Result<String> {
    env::var(key)
        .with_context(|| format!("缺少环境变量: {key}"))
        .and_then(|v| {
            if v.trim().is_empty() {
                Err(anyhow!("环境变量为空: {key}"))
            } else {
                Ok(v)
            }
        })
}

pub fn env_or(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

pub fn env_parse_or<T>(key: &str, default: T) -> Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(v) if !v.trim().is_empty() => v
            .parse::<T>()
            .map_err(|e| anyhow!("环境变量解析失败 {key}={v:?}: {e}")),
        _ => Ok(default),
    }
}

