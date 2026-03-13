use anyhow::{Result, anyhow};

use crate::infrastructure::{
    config::env_reader::{env_parse_or, required_env},
    school::config::{AhutGatewayConfig, normalize_base_url},
};

fn parse_user_agent_pool(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|it| it.trim())
        .filter(|it| !it.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[derive(Debug, Clone)]
pub struct SchoolSecurityConfig {
    pub master_key: String,
}

#[derive(Debug, Clone)]
pub struct SchoolPolicyEnvConfig {
    pub refresh_skew_secs: i64,
    pub auth_max_retry_times: u32,
    pub biz_max_retry_times: u32,
}

#[derive(Debug, Clone)]
pub struct SchoolInfraConfig {
    pub gateway: AhutGatewayConfig,
    pub security: SchoolSecurityConfig,
    pub policy: SchoolPolicyEnvConfig,
    pub school_fixed_authorization: String,
}

impl SchoolInfraConfig {
    pub fn from_env() -> Result<Self> {
        let raw_base_url = required_env("AHUT_BASE_URL")?;
        let master_key = required_env("SCHOOL_CREDENTIAL_MASTER_KEY")?;
        let fallback_user_agent = required_env("HTTP_FALLBACK_USER_AGENT")?;
        let default_user_agent_pool = parse_user_agent_pool(&required_env("HTTP_DEFAULT_USER_AGENT_POOL")?);

        if master_key.trim().is_empty() {
            return Err(anyhow!("环境变量为空: SCHOOL_CREDENTIAL_MASTER_KEY"));
        }
        if default_user_agent_pool.is_empty() {
            return Err(anyhow!("环境变量为空或无可用UA: HTTP_DEFAULT_USER_AGENT_POOL"));
        }

        Ok(Self {
            gateway: AhutGatewayConfig {
                base_url: normalize_base_url(&raw_base_url),
                fallback_user_agent,
                default_user_agent_pool,
                connect_timeout: std::time::Duration::from_millis(env_parse_or(
                    "HTTP_CONNECT_TIMEOUT_MS",
                    3000u64,
                )?),
                request_timeout: std::time::Duration::from_millis(env_parse_or(
                    "HTTP_REQUEST_TIMEOUT_MS",
                    10000u64,
                )?),
                pool_idle_timeout: std::time::Duration::from_secs(env_parse_or(
                    "HTTP_POOL_IDLE_TIMEOUT_SECS",
                    60u64,
                )?),
                pool_max_idle_per_host: env_parse_or("HTTP_POOL_MAX_IDLE_PER_HOST", 16usize)?,
                tcp_keepalive: std::time::Duration::from_secs(env_parse_or(
                    "HTTP_TCP_KEEPALIVE_SECS",
                    30u64,
                )?),
            },
            security: SchoolSecurityConfig { master_key },
            policy: SchoolPolicyEnvConfig {
                refresh_skew_secs: env_parse_or("SCHOOL_AUTH_TOKEN_REFRESH_SKEW_SECS", 300)?,
                auth_max_retry_times: env_parse_or("SCHOOL_AUTH_MAX_RETRY_TIMES", 2)?,
                biz_max_retry_times: env_parse_or("SCHOOL_BIZ_MAX_RETRY_TIMES", 2)?,
            },
            school_fixed_authorization: required_env("SCHOOL_FIXED_AUTHORIZATION")?,
        })
    }
}
