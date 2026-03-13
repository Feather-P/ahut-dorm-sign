use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, NaiveTime, Utc};
use reqwest::{Client, Url, header};
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

use super::gateway_support::{
    ApiResp, SCHOOL_TIME_ZONE, TaskListData, TokenResp, build_wechat_headers, ensure_api_success,
    map_transport_err, map_upstream_rejected, token_expired_at,
};
use super::week_mapper::{parse_school_week, to_school_week};

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
            .header(
                header::AUTHORIZATION,
                self.school_fixed_authorization.clone(),
            )
            .send()
            .await
            .map_err(|_| map_transport_err())?;

        let body = resp
            .json::<TokenResp>()
            .await
            .map_err(|_| map_transport_err())?;

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
        let expired_at = token_expired_at(now_utc, expires_in);
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
            .header(
                header::AUTHORIZATION,
                self.school_fixed_authorization.clone(),
            )
            .send()
            .await
            .map_err(|_| map_transport_err())?;

        let body = resp
            .json::<TokenResp>()
            .await
            .map_err(|_| map_transport_err())?;

        let access_token = body.access_token.ok_or(DomainError::TokenExpired {
            origin: ErrorSource::School,
        })?;
        let refresh_token = body.refresh_token.unwrap_or_else(|| access_token.clone());
        let expires_in = body.expires_in.unwrap_or(7200);
        let now_utc = Utc::now();
        let expired_at = token_expired_at(now_utc, expires_in);
        SchoolToken::new(access_token, refresh_token, expired_at)
    }

    async fn fetch_active_task_list(
        &self,
        session: &SchoolSession,
        selected_ua: &str,
    ) -> Result<Vec<SchoolSignTask>, DomainError> {
        let url = self.api_url(
            "flySource-yxgl/dormSignTask/getStudentTaskPage?userDataType=student&current=1&size=15",
        )?;
        let now_utc = Utc::now();
        // 学校业务接口签名与 flysource-auth 实测应使用 refresh_token。
        let biz_token = session.refresh_token().trim();
        let headers = build_wechat_headers(
            session,
            &self.school_fixed_authorization,
            selected_ua,
            self.signer.auth(biz_token),
            self.signer.sign(biz_token, url.as_str(), now_utc),
            Some(format!(
                "https://xskq.ahut.edu.cn/wise/pages/ssgl/dormsign?&userId={}",
                session.student_id()
            )),
        );

        let resp = self
            .client
            .get(url)
            .headers(headers)
            .send()
            .await
            .map_err(|_| map_transport_err())?;

        let body = ensure_api_success(
            resp.json::<ApiResp<TaskListData>>()
                .await
                .map_err(|_| map_transport_err())?,
        )?;

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
        selected_ua: &str,
    ) -> Result<(), DomainError> {
        let biz_token = session.refresh_token().trim();
        let wechat_url = self.api_url(&format!(
            "flySource-base/wechat/getWechatMpConfig?configUrl=https://xskq.ahut.edu.cn/wise/pages/ssgl/dormsign?taskId={task_id}&autoSign=1&scanSign=0&userId={}",
            session.student_id()
        ))?;
        let wechat_headers = build_wechat_headers(
            session,
            &self.school_fixed_authorization,
            selected_ua,
            self.signer.auth(biz_token),
            self.signer.sign(biz_token, wechat_url.as_str(), Utc::now()),
            None,
        );

        let wechat_resp = self
            .client
            .get(wechat_url.clone())
            .headers(wechat_headers)
            .send()
            .await
            .map_err(|_| map_transport_err())?;

        let _ = ensure_api_success(
            wechat_resp
                .json::<ApiResp<serde_json::Value>>()
                .await
                .map_err(|_| map_transport_err())?,
        )?;

        let api_log_url = self
            .api_url("flySource-base/apiLog/save?menuTitle=%E6%99%9A%E5%AF%9D%E7%AD%BE%E5%88%B0")?;
        let api_log_headers = build_wechat_headers(
            session,
            &self.school_fixed_authorization,
            selected_ua,
            self.signer.auth(biz_token),
            self.signer
                .sign(biz_token, api_log_url.as_str(), Utc::now()),
            None,
        );
        let api_log_resp = self
            .client
            .post(api_log_url.clone())
            .headers(api_log_headers)
            .send()
            .await
            .map_err(|_| map_transport_err())?;

        if !api_log_resp.status().is_success() {
            return Err(map_upstream_rejected(
                api_log_resp.status().as_u16() as i64,
                "apiLog 请求失败".to_string(),
            ));
        }

        Ok(())
    }

    async fn submit_checkin(
        &self,
        session: &SchoolSession,
        check_cmd: CheckinCommand,
        selected_ua: &str,
    ) -> Result<(), DomainError> {
        let url = self.api_url("flySource-yxgl/dormSignRecord/add")?;
        let now_utc = Utc::now();
        let biz_token = session.refresh_token().trim();
        let local = check_cmd.occurred_at_utc().with_timezone(&SCHOOL_TIME_ZONE);
        let submit_headers = build_wechat_headers(
            session,
            &self.school_fixed_authorization,
            selected_ua,
            self.signer.auth(biz_token),
            self.signer.sign(biz_token, url.as_str(), now_utc),
            None,
        );

        let payload = serde_json::json!({
            "taskId": check_cmd.task_id(),
            "signAddress": "",
            "locationAccuracy": check_cmd.accuracy_meters(),
            "signLat": check_cmd.point().lat(),
            "signLng": check_cmd.point().lng(),
            "signType": 0,
            "fileId": "",
            // 学校这里确实固定写着的这个路径，但名字是Base64是何意味？
            "imgBase64": "/static/images/dormitory/photo.png",
            "signDate": local.format("%Y-%m-%d").to_string(),
            "signTime": local.format("%H:%M:%S").to_string(),
            "signWeek": to_school_week(local.weekday()),
            "scanCode": "",
        });

        let resp = self
            .client
            .post(url.clone())
            .headers(submit_headers)
            .json(&payload)
            .send()
            .await
            .map_err(|_| map_transport_err())?;

        let body = resp
            .json::<ApiResp<serde_json::Value>>()
            .await
            .map_err(|_| map_transport_err())?;

        if body.code == 200 || body.msg.contains("已完成签到") {
            return Ok(());
        }

        Err(map_upstream_rejected(body.code, body.msg))
    }
}
