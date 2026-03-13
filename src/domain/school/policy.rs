use chrono::{DateTime, Duration, Utc};

use crate::domain::{
    error::{DomainError, ErrorKind},
    school::session::SchoolSession,
};

#[derive(Debug, Clone)]
pub struct SchoolPolicyConfig {
    auth: SchoolAuthPolicyConfig,
    business: SchoolBusinessPolicyConfig,
}

impl SchoolPolicyConfig {
    pub fn new(auth: SchoolAuthPolicyConfig, business: SchoolBusinessPolicyConfig) -> Self {
        Self { auth, business }
    }

    pub fn auth(&self) -> &SchoolAuthPolicyConfig {
        &self.auth
    }

    pub fn business(&self) -> &SchoolBusinessPolicyConfig {
        &self.business
    }
}

#[derive(Debug, Clone)]
pub struct SchoolAuthPolicyConfig {
    token_need_refresh_before_expired_duration: Duration,
    max_retry_times: u32,
}

impl SchoolAuthPolicyConfig {
    pub fn new(
        token_need_refresh_before_expired_duration: Duration,
        max_retry_times: u32,
    ) -> Result<Self, DomainError> {
        if token_need_refresh_before_expired_duration < Duration::zero() {
            return Err(DomainError::InvalidTokenRefreshSkewSeconds(
                token_need_refresh_before_expired_duration.num_seconds(),
            ));
        }

        if max_retry_times == 0 {
            return Err(DomainError::InvalidRetryTimes(max_retry_times));
        }

        Ok(Self {
            token_need_refresh_before_expired_duration,
            max_retry_times,
        })
    }

    pub fn token_need_refresh_before_expired_duration(&self) -> Duration {
        self.token_need_refresh_before_expired_duration
    }

    pub fn max_retry_times(&self) -> u32 {
        self.max_retry_times
    }
}

#[derive(Debug, Clone)]
pub struct SchoolBusinessPolicyConfig {
    max_retry_times: u32,
}

impl SchoolBusinessPolicyConfig {
    pub fn new(max_retry_times: u32) -> Result<Self, DomainError> {
        if max_retry_times == 0 {
            return Err(DomainError::InvalidRetryTimes(max_retry_times));
        }

        Ok(Self { max_retry_times })
    }

    pub fn max_retry_times(&self) -> u32 {
        self.max_retry_times
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusinessDecision {
    Retry,
    Stop,
    Success,
}

pub enum AuthDecision {
    UseCurrentToken,
    RefreshToken,
    ReAuthenticate,
}

#[derive(Debug, Clone)]
pub struct SchoolAuthDecider {
    config: SchoolAuthPolicyConfig,
}

impl SchoolAuthDecider {
    pub fn new(config: SchoolAuthPolicyConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &SchoolAuthPolicyConfig {
        &self.config
    }

    /// 基于session 的 token 状态决定认证动作。
    pub fn decide_by_session(
        &self,
        session: Option<SchoolSession>,
        utc_now: DateTime<Utc>,
    ) -> AuthDecision {
        match session {
            None => return AuthDecision::ReAuthenticate,
            Some(session) => {
                if session.need_refresh(
                    utc_now,
                    self.config.token_need_refresh_before_expired_duration(),
                ) {
                    return AuthDecision::RefreshToken;
                } else {
                    return AuthDecision::UseCurrentToken;
                }
            }
        }
    }

    /// 基于上一次错误语义决定下一步认证动作。
    pub fn decide_after_error(&self, err: &DomainError) -> AuthDecision {
        match err.kind() {
            ErrorKind::ReauthRequired => AuthDecision::ReAuthenticate,
            ErrorKind::Retryable => AuthDecision::RefreshToken,
            ErrorKind::Terminal | ErrorKind::IdempotentSuccess => AuthDecision::UseCurrentToken,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SchoolBusinessDecider {
    config: SchoolBusinessPolicyConfig,
}

impl SchoolBusinessDecider {
    pub fn new(config: SchoolBusinessPolicyConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &SchoolBusinessPolicyConfig {
        &self.config
    }

    /// 基于错误语义和当前重试次数决定业务动作。
    ///
    /// - `attempted_retry_times`: 已经发生过的重试次数（从 0 开始）
    pub fn decide_after_error(
        &self,
        err: &DomainError,
        attempted_retry_times: u32,
    ) -> BusinessDecision {
        match err.kind() {
            ErrorKind::IdempotentSuccess => BusinessDecision::Success,
            ErrorKind::Retryable => {
                if attempted_retry_times < self.config.max_retry_times() {
                    BusinessDecision::Retry
                } else {
                    BusinessDecision::Stop
                }
            }
            ErrorKind::ReauthRequired | ErrorKind::Terminal => BusinessDecision::Stop,
        }
    }
}
