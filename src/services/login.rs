use crate::constants::auth;
use crate::constants::endpoints::LOGIN;
use crate::error::{AppError, ServiceError};
use crate::models::auth::AuthInfo;
use crate::transport::{AppClient, HttpMethod};
use crate::utils::hash::encode_md5;
use crate::utils::headers::insert_header_str;
use derive_builder::Builder;
use reqwest::header::{AUTHORIZATION as HEADER_AUTHORIZATION, HeaderMap};
use std::sync::Arc;

/// 登录请求参数
#[derive(Debug, serde::Serialize, Builder)]
#[builder(default)]
pub struct LoginRequest {
    /// 校区号
    pub tenant_id: String,
    /// 学号
    pub username: String,
    /// 密码
    pub password: String,
    /// 账号类型
    #[serde(rename = "type")]
    pub r#type: String,
    /// 认证类型
    pub grant_type: String,
    /// 作用域
    pub scope: String,
}

impl LoginRequest {
    pub fn new(
        tenant_id: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        r#type: impl Into<String>,
        grant_type: impl Into<String>,
        scope: impl Into<String>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            username: username.into(),
            password: password.into(),
            r#type: r#type.into(),
            grant_type: grant_type.into(),
            scope: scope.into(),
        }
    }
}

impl Default for LoginRequest {
    fn default() -> Self {
        Self {
            tenant_id: String::from(auth::TENANT_ID),
            username: String::new(),
            password: String::new(),
            r#type: String::from(auth::TYPE),
            grant_type: String::from(auth::GRANT_TYPE),
            scope: String::from(auth::SCOPE),
        }
    }
}

impl From<LoginRequestBuilderError> for ServiceError {
    fn from(err: LoginRequestBuilderError) -> Self {
        ServiceError::BuildError {
            service: "login.login_request_builder",
            msg: err.to_string(),
        }
    }
}

/// 登录服务
pub struct LoginService {
    client: Arc<AppClient>,
}

impl LoginService {
    /// 创建新的登录服务
    pub fn new(client: Arc<AppClient>) -> Self {
        Self { client }
    }

    /// 执行登录
    async fn login(
        &self,
        tenant_id: &str,
        username: &str,
        password: &str,
        r#type: &str,
        grant_type: &str,
        scope: &str,
    ) -> Result<AuthInfo, AppError> {
        let request = LoginRequestBuilder::default()
            .tenant_id(tenant_id.to_string())
            .username(username.to_string())
            .password(encode_md5(password))
            .r#type(r#type.to_string())
            .grant_type(grant_type.to_string())
            .scope(scope.to_string())
            .build()
            .map_err(ServiceError::from)?;

        let mut headers = HeaderMap::with_capacity(1);
        insert_header_str(
            &mut headers,
            HEADER_AUTHORIZATION,
            auth::LOGIN_AUTHORIZATION,
        )
        .map_err(|e| ServiceError::InvalidRequest {
            service: "login",
            msg: e.to_string(),
        })?;
        insert_header_str(&mut headers, "Tenant-Id", &request.tenant_id).map_err(|e| {
            ServiceError::InvalidRequest {
                service: "login",
                msg: e.to_string(),
            }
        })?;

        let response = self
            .client
            .request(HttpMethod::Post, LOGIN)
            .header_map(headers)
            .query(&request)?
            .send()
            .await?;

        self.client
            .parse_json::<AuthInfo>(response)
            .await
            .map_err(AppError::from)
    }

    /// 使用默认参数执行登录（tenant_id/type/grant_type/scope 来自常量）
    pub async fn login_with_default(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthInfo, AppError> {
        let request = LoginRequest::default();
        self.login(
            &request.tenant_id,
            username,
            password,
            &request.r#type,
            &request.grant_type,
            &request.scope,
        )
        .await
    }
}
