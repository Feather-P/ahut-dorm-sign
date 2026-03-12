use std::{env, str::FromStr, time::Duration};

use anyhow::{Context, Result, anyhow};
use dotenvy::dotenv;
use reqwest::Client;

use crate::domain::school::policy::{
    SchoolAuthPolicyConfig, SchoolBusinessPolicyConfig, SchoolPolicyConfig,
};

#[derive(Debug, Clone)]
pub struct AhutGatewayConfig {
    pub base_url: String,
    pub user_agent: String,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub pool_idle_timeout: Duration,
    pub pool_max_idle_per_host: usize,
    pub tcp_keepalive: Duration,
}

impl AhutGatewayConfig {
    pub fn from_env() -> Result<Self> {
        let _ = dotenv();

        let raw_base_url = required_env("AHUT_BASE_URL")?;
        let base_url = normalize_base_url(&raw_base_url);

        Ok(Self {
            base_url,
            user_agent: env_or("HTTP_USER_AGENT", "ahut-dorm-sign/0.1"),
            connect_timeout: Duration::from_millis(env_parse_or("HTTP_CONNECT_TIMEOUT_MS", 3000u64)?),
            request_timeout: Duration::from_millis(env_parse_or("HTTP_REQUEST_TIMEOUT_MS", 10000u64)?),
            pool_idle_timeout: Duration::from_secs(env_parse_or("HTTP_POOL_IDLE_TIMEOUT_SECS", 60u64)?),
            pool_max_idle_per_host: env_parse_or("HTTP_POOL_MAX_IDLE_PER_HOST", 16usize)?,
            tcp_keepalive: Duration::from_secs(env_parse_or("HTTP_TCP_KEEPALIVE_SECS", 30u64)?),
        })
    }

    pub fn build_client(&self) -> Result<Client> {
        Client::builder()
            .connect_timeout(self.connect_timeout)
            .timeout(self.request_timeout)
            .pool_idle_timeout(self.pool_idle_timeout)
            .pool_max_idle_per_host(self.pool_max_idle_per_host)
            .tcp_keepalive(self.tcp_keepalive)
            .user_agent(self.user_agent.clone())
            .build()
            .context("构建 reqwest::Client 失败")
    }
}

pub fn school_policy_from_env() -> Result<SchoolPolicyConfig> {
    let _ = dotenv();

    let refresh_skew_secs: i64 = env_parse_or("SCHOOL_AUTH_TOKEN_REFRESH_SKEW_SECS", 300)?;
    let auth_max_retry_times: u32 = env_parse_or("SCHOOL_AUTH_MAX_RETRY_TIMES", 2)?;
    let biz_max_retry_times: u32 = env_parse_or("SCHOOL_BIZ_MAX_RETRY_TIMES", 2)?;

    let auth = SchoolAuthPolicyConfig::new(
        chrono::Duration::seconds(refresh_skew_secs),
        auth_max_retry_times,
    )
    .map_err(|e| anyhow!("加载 SCHOOL_AUTH_* 策略失败: {e}"))?;

    let business = SchoolBusinessPolicyConfig::new(biz_max_retry_times)
        .map_err(|e| anyhow!("加载 SCHOOL_BIZ_* 策略失败: {e}"))?;

    Ok(SchoolPolicyConfig::new(auth, business))
}

fn required_env(key: &str) -> Result<String> {
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

fn env_or(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_parse_or<T>(key: &str, default: T) -> Result<T>
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

fn normalize_base_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.ends_with("/api") {
        format!("{trimmed}/")
    } else {
        format!("{trimmed}/api/")
    }
}
