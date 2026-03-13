use chrono::{DateTime, Utc};
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::Deserialize;

use crate::domain::{
    error::{DomainError, ErrorSource},
    school::session::SchoolSession,
};

/// 学校侧业务时间固定使用 Asia/Shanghai。
///
/// 约定：
/// 1) 领域层/存储层统一传递 UTC（DateTime<Utc> / PostgreSQL timestamptz）。
/// 2) 仅在和学校 API 交互时做时区转换。
pub const SCHOOL_TIME_ZONE: chrono_tz::Tz = chrono_tz::Asia::Shanghai;

#[derive(Debug, Deserialize)]
pub struct TokenResp {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiResp<T> {
    pub code: i64,
    pub msg: String,
    pub data: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct TaskListData {
    pub records: Vec<TaskItem>,
}

#[derive(Debug, Deserialize)]
pub struct TaskItem {
    #[serde(rename = "taskId")]
    pub task_id: String,
    #[serde(rename = "taskName")]
    pub task_name: String,
    #[serde(rename = "taskStartDate")]
    pub task_start_date: String,
    #[serde(rename = "taskEndDate")]
    pub task_end_date: String,
    #[serde(rename = "signStartTime")]
    pub sign_start_time: String,
    #[serde(rename = "signEndTime")]
    pub sign_end_time: String,
    #[serde(rename = "signWeek")]
    pub sign_week: String,
}

pub fn map_transport_err() -> DomainError {
    DomainError::RemoteUnavailable {
        origin: ErrorSource::School,
    }
}

pub fn map_upstream_rejected(code: i64, msg: String) -> DomainError {
    DomainError::UpstreamRejected {
        origin: ErrorSource::School,
        code: Some(code),
        message: msg,
    }
}

pub fn ensure_api_success<T>(resp: ApiResp<T>) -> Result<ApiResp<T>, DomainError> {
    if resp.code == 200 {
        Ok(resp)
    } else {
        Err(map_upstream_rejected(resp.code, resp.msg))
    }
}

pub fn build_wechat_headers(
    session: &SchoolSession,
    school_fixed_authorization: &str,
    user_agent: &str,
    signer_auth: String,
    signer_sign: String,
    referer: Option<String>,
) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(school_fixed_authorization)
            .unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    if let Ok(v) = HeaderValue::from_str(user_agent) {
        headers.insert(header::USER_AGENT, v);
    }
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json;charset=UTF-8"),
    );
    headers.insert(
        "X-Requested-With",
        HeaderValue::from_static("com.tencent.mm"),
    );
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://xskq.ahut.edu.cn"),
    );
    if let Some(referer) = referer {
        if let Ok(v) = HeaderValue::from_str(&referer) {
            headers.insert("Referer", v);
        }
    }
    if let Ok(v) = HeaderValue::from_str(&signer_auth) {
        headers.insert("flysource-auth", v);
    }
    if let Ok(v) = HeaderValue::from_str(&signer_sign) {
        headers.insert("flysource-sign", v);
    }
    let _ = session;
    headers
}

pub fn token_expired_at(now_utc: DateTime<Utc>, expires_in_secs: i64) -> DateTime<Utc> {
    now_utc + chrono::Duration::seconds(expires_in_secs)
}
