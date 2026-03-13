use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;

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

pub(crate) fn normalize_base_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.ends_with("/api") {
        format!("{trimmed}/")
    } else {
        format!("{trimmed}/api/")
    }
}
