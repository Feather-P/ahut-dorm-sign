use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, NaiveTime, Utc};
use reqwest::{Client, Url, header};
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::{
    error::{DomainError, ErrorSource},
    school::{
        crypto::SchoolCredentialProtector,
        gateway::SchoolGateway,
        session::SchoolSession,
        sign::SchoolSignGenerator,
        task::{CheckinCommand, DateRange, SchoolSignTask, TimeWindow},
        token::SchoolToken,
        user::SchoolUser,
    },
};

use super::week_mapper::{parse_school_week, to_school_week};

/// 学校侧业务时间固定使用 Asia/Shanghai。
///
/// 约定：
/// 1) 领域层/存储层统一传递 UTC（DateTime<Utc> / PostgreSQL timestamptz）。
/// 2) 仅在和学校 API 交互时做时区转换。
const SCHOOL_TIME_ZONE: chrono_tz::Tz = chrono_tz::Asia::Shanghai;

pub struct AhutGateway {
    client: Client,
    base_url: Url,
    school_fixed_authorization: String,
    credential_protector: Box<dyn SchoolCredentialProtector + Send + Sync>,
    signer: Box<dyn SchoolSignGenerator + Send + Sync>,
}

impl AhutGateway {
    pub fn new(
        client: Client,
        base_url: Url,
        school_fixed_authorization: String,
        credential_protector: Box<dyn SchoolCredentialProtector + Send + Sync>,
        signer: Box<dyn SchoolSignGenerator + Send + Sync>,
    ) -> Self {
        Self {
            client,
            base_url,
            school_fixed_authorization,
            credential_protector,
            signer,
        }
    }

    fn api_url(&self, path: &str) -> Result<Url, DomainError> {
        self.base_url
            .join(path)
            .map_err(|_| DomainError::UpstreamRejected {
                origin: ErrorSource::School,
                code: None,
                message: format!("url 拼接失败: {path}"),
            })
    }
}

#[derive(Debug, Deserialize)]
struct TokenResp {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiResp<T> {
    code: i64,
    msg: String,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct TaskListData {
    records: Vec<TaskItem>,
}

#[derive(Debug, Deserialize)]
struct TaskItem {
    #[serde(rename = "taskId")]
    task_id: String,
    #[serde(rename = "taskName")]
    task_name: String,
    #[serde(rename = "taskStartDate")]
    task_start_date: String,
    #[serde(rename = "taskEndDate")]
    task_end_date: String,
    #[serde(rename = "signStartTime")]
    sign_start_time: String,
    #[serde(rename = "signEndTime")]
    sign_end_time: String,
    #[serde(rename = "signWeek")]
    sign_week: String,
}

#[async_trait]
impl SchoolGateway for AhutGateway {
    async fn authenticate(&self, user: &SchoolUser) -> Result<SchoolToken, DomainError> {
        let url = self.api_url("flySource-auth/oauth/token")?;
        let school_auth_secret = user.decrypt_credential(self.credential_protector.as_ref())?;

        let resp = self
            .client
            .post(url)
            .query(&[
                ("tenantId", "000000"),
                ("username", user.student_id()),
                ("password", school_auth_secret.as_str()),
                ("type", "account"),
                ("grant_type", "password"),
                ("scope", "all"),
            ])
            .header(header::AUTHORIZATION, self.school_fixed_authorization.clone())
            .send()
            .await
            .map_err(|_| DomainError::RemoteUnavailable {
                origin: ErrorSource::School,
            })?;

        let body = resp
            .json::<TokenResp>()
            .await
            .map_err(|_| DomainError::RemoteUnavailable {
                origin: ErrorSource::School,
            })?;

        let access_token = body.access_token.ok_or_else(|| {
            if body
                .error_description
                .as_deref()
                .unwrap_or_default()
                .contains("Bad credentials")
            {
                DomainError::Unauthorized {
                    origin: ErrorSource::School,
                }
            } else {
                DomainError::UpstreamRejected {
                    origin: ErrorSource::School,
                    code: None,
                    message: body
                        .error_description
                        .unwrap_or_else(|| "认证失败：未返回 access_token".to_string()),
                }
            }
        })?;

        let refresh_token = body.refresh_token.unwrap_or_else(|| access_token.clone());
        let expires_in = body.expires_in.unwrap_or(3600);
        let now_utc = Utc::now();
        let expired_at = now_utc + chrono::Duration::seconds(expires_in);
        SchoolToken::new(access_token, refresh_token, expired_at)
    }

    async fn refresh(&self, session: &SchoolSession) -> Result<SchoolToken, DomainError> {
        let url = self.api_url("flySource-auth/oauth/token")?;
        let resp = self
            .client
            .post(url)
            .query(&[
                ("tenantId", "000000"),
                ("grant_type", "refresh_token"),
                ("scope", "all"),
                ("refresh_token", session.refresh_token()),
            ])
            .header(header::AUTHORIZATION, self.school_fixed_authorization.clone())
            .send()
            .await
            .map_err(|_| DomainError::RemoteUnavailable {
                origin: ErrorSource::School,
            })?;

        let body = resp
            .json::<TokenResp>()
            .await
            .map_err(|_| DomainError::RemoteUnavailable {
                origin: ErrorSource::School,
            })?;

        let access_token = body.access_token.ok_or(DomainError::TokenExpired {
            origin: ErrorSource::School,
        })?;
        let refresh_token = body.refresh_token.unwrap_or_else(|| access_token.clone());
        let expires_in = body.expires_in.unwrap_or(7200);
        let now_utc = Utc::now();
        let expired_at = now_utc + chrono::Duration::seconds(expires_in);
        SchoolToken::new(access_token, refresh_token, expired_at)
    }

    async fn fetch_active_task_list(
        &self,
        session: &SchoolSession,
    ) -> Result<Vec<SchoolSignTask>, DomainError> {
        let url = self.api_url(
            "flySource-yxgl/dormSignTask/getStudentTaskPage?userDataType=student&current=1&size=15",
        )?;
        let now_utc = Utc::now();
        let flysource_sign = self
            .signer
            .sign(session.access_token(), url.as_str(), now_utc);

        let resp = self
            .client
            .get(url)
            .header(header::AUTHORIZATION, self.school_fixed_authorization.clone())
            .header("FlySource-Auth", self.signer.auth(session.access_token()))
            .header("FlySource-sign", flysource_sign)
            .send()
            .await
            .map_err(|_| DomainError::RemoteUnavailable {
                origin: ErrorSource::School,
            })?;

        let body = resp
            .json::<ApiResp<TaskListData>>()
            .await
            .map_err(|_| DomainError::RemoteUnavailable {
                origin: ErrorSource::School,
            })?;

        if body.code != 200 {
            return Err(DomainError::UpstreamRejected {
                origin: ErrorSource::School,
                code: Some(body.code),
                message: body.msg,
            });
        }

        let tasks = body
            .data
            .unwrap_or(TaskListData { records: vec![] })
            .records
            .into_iter()
            .map(|it| {
                let start_date = NaiveDate::parse_from_str(&it.task_start_date, "%Y-%m-%d")
                    .map_err(|_| DomainError::InvalidDateRange)?;
                let end_date = NaiveDate::parse_from_str(&it.task_end_date, "%Y-%m-%d")
                    .map_err(|_| DomainError::InvalidDateRange)?;
                let start_time = NaiveTime::parse_from_str(&it.sign_start_time, "%H:%M")
                    .map_err(|_| DomainError::InvalidTimeWindow)?;
                let end_time = NaiveTime::parse_from_str(&it.sign_end_time, "%H:%M")
                    .map_err(|_| DomainError::InvalidTimeWindow)?;

                SchoolSignTask::new(
                    Uuid::new_v4(),
                    session.student_id().to_string(),
                    it.task_id,
                    it.task_name,
                    DateRange::new(start_date, end_date)?,
                    TimeWindow::new(start_time, end_time)?,
                    parse_school_week(&it.sign_week),
                    SCHOOL_TIME_ZONE,
                )
            })
            .collect::<Result<Vec<_>, DomainError>>()?;

        Ok(tasks)
    }

    async fn prepare_checkin_context(
        &self,
        session: &SchoolSession,
        task_id: &str,
    ) -> Result<(), DomainError> {
        let wechat_url = self.api_url(&format!(
            "flySource-base/wechat/getWechatMpConfig?configUrl=https://xskq.ahut.edu.cn/wise/pages/ssgl/dormsign?taskId={task_id}&autoSign=1&scanSign=0&userId={}",
            session.student_id()
        ))?;

        let wechat_resp = self
            .client
            .get(wechat_url.clone())
            .header(header::AUTHORIZATION, self.school_fixed_authorization.clone())
            .header("FlySource-Auth", self.signer.auth(session.access_token()))
            .header(
                "FlySource-sign",
                self.signer
                    .sign(session.access_token(), wechat_url.as_str(), Utc::now()),
            )
            .send()
            .await
            .map_err(|_| DomainError::RemoteUnavailable {
                origin: ErrorSource::School,
            })?;

        let wechat_body = wechat_resp
            .json::<ApiResp<serde_json::Value>>()
            .await
            .map_err(|_| DomainError::RemoteUnavailable {
                origin: ErrorSource::School,
            })?;

        if wechat_body.code != 200 {
            return Err(DomainError::UpstreamRejected {
                origin: ErrorSource::School,
                code: Some(wechat_body.code),
                message: wechat_body.msg,
            });
        }

        let api_log_url = self.api_url(
            "flySource-base/apiLog/save?menuTitle=%E6%99%9A%E5%AF%9D%E7%AD%BE%E5%88%B0",
        )?;
        let api_log_resp = self
            .client
            .post(api_log_url.clone())
            .header(header::AUTHORIZATION, self.school_fixed_authorization.clone())
            .header("FlySource-Auth", self.signer.auth(session.access_token()))
            .header(
                "FlySource-sign",
                self.signer
                    .sign(session.access_token(), api_log_url.as_str(), Utc::now()),
            )
            .send()
            .await
            .map_err(|_| DomainError::RemoteUnavailable {
                origin: ErrorSource::School,
            })?;

        if !api_log_resp.status().is_success() {
            return Err(DomainError::UpstreamRejected {
                origin: ErrorSource::School,
                code: Some(api_log_resp.status().as_u16() as i64),
                message: "apiLog 请求失败".to_string(),
            });
        }

        Ok(())
    }

    async fn submit_checkin(
        &self,
        session: &SchoolSession,
        target: CheckinCommand,
    ) -> Result<(), DomainError> {
        let url = self.api_url("flySource-yxgl/dormSignRecord/add")?;
        let now_utc = Utc::now();
        let local = target
            .occurred_at_utc()
            .with_timezone(&SCHOOL_TIME_ZONE);

        let payload = serde_json::json!({
            "taskId": target.task_id(),
            "signAddress": "",
            "locationAccuracy": target.accuracy_meters(),
            "signLat": target.point().lat(),
            "signLng": target.point().lng(),
            "signType": 0,
            "fileId": "",
            "imgBase64": "/static/images/dormitory/photo.png",
            "signDate": local.format("%Y-%m-%d").to_string(),
            "signTime": local.format("%H:%M:%S").to_string(),
            "signWeek": to_school_week(local.weekday()),
            "scanCode": "",
        });

        let resp = self
            .client
            .post(url.clone())
            .header(header::AUTHORIZATION, self.school_fixed_authorization.clone())
            .header("FlySource-Auth", self.signer.auth(session.access_token()))
            .header(
                "FlySource-sign",
                self.signer
                    .sign(session.access_token(), url.as_str(), now_utc),
            )
            .json(&payload)
            .send()
            .await
            .map_err(|_| DomainError::RemoteUnavailable {
                origin: ErrorSource::School,
            })?;

        let body = resp
            .json::<ApiResp<serde_json::Value>>()
            .await
            .map_err(|_| DomainError::RemoteUnavailable {
                origin: ErrorSource::School,
            })?;

        if body.code == 200 || body.msg.contains("已完成签到") {
            return Ok(());
        }

        Err(DomainError::UpstreamRejected {
            origin: ErrorSource::School,
            code: Some(body.code),
            message: body.msg,
        })
    }
}
