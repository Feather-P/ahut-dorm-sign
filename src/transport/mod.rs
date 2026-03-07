use crate::constants::client::{DEFAULT_TIMEOUT_SECS, DEFAULT_USER_AGENT};
use crate::error::{AppError, TransportError};
use crate::models::envelope::BizEnvelope;
use crate::utils::sign::build_app_signed_headers;
use crate::utils::url::join_base_and_path;
use reqwest::{Client, header::HeaderMap};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// App 客户端构建器
#[derive(Debug, Clone)]
pub struct AppClientBuilder {
    base_url: String,
    timeout: Duration,
    user_agent: String,
}

impl AppClientBuilder {
    /// 创建构建器
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            user_agent: DEFAULT_USER_AGENT.to_string(),
        }
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 设置用户代理
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// 构建 App 客户端
    pub fn build(self) -> Result<AppClient, TransportError> {
        let http = Client::builder()
            .timeout(self.timeout)
            .user_agent(&self.user_agent)
            .build()
            .map_err(|e| TransportError::ClientBuildError(e.to_string()))?;

        Ok(AppClient {
            http,
            base_url: self.base_url,
        })
    }
}

/// App 客户端
#[derive(Clone)]
pub struct AppClient {
    pub(crate) http: Client,
    base_url: String,
}

/// App 的 http 请求方法
#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Get,
    Post,
}

/// 统一请求构建器
pub struct RequestBuilder<'a> {
    client: &'a AppClient,
    method: HttpMethod,
    path: String,
    request: reqwest::RequestBuilder,
    headers: Option<HeaderMap>,
    token: Option<String>,
}

impl<'a> RequestBuilder<'a> {
    pub fn header_map(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    pub fn query<T: serde::Serialize + ?Sized>(mut self, query: &T) -> Self {
        self.request = self.request.query(query);
        self
    }

    pub fn sign(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    pub fn json<T: serde::Serialize + ?Sized>(mut self, body: &T) -> Self {
        self.request = self.request.json(body);
        self
    }

    pub async fn send(self) -> Result<reqwest::Response, TransportError> {
        let url = self.client.full_url(&self.path);
        let method = match self.method {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
        };
        info!(
            step = "transport.send",
            method,
            path = %self.path,
            has_token = self.token.is_some(),
            "sending http request"
        );

        let mut request = self.request;

        if let Some(headers) = self.headers {
            request = request.headers(headers);
        }

        // 存在token时的签名逻辑
        match self.token {
            Some(tok) => {
                let signed_headers = build_app_signed_headers(&url, &tok).map_err(|e| {
                    TransportError::ClientBuildError(format!("签名请求头构造失败: {}", e))
                })?;
                request = request.headers(signed_headers);
            }
            None => {}
        }

        let response = request.send().await?;
        debug!(
            step = "transport.response.received",
            method,
            path = %self.path,
            status = response.status().as_u16(),
            "http response received"
        );
        self.client.handle_response(response).await
    }
}

impl AppClient {
    /// 创建客户端构建器
    pub fn builder(base_url: impl Into<String>) -> AppClientBuilder {
        AppClientBuilder::new(base_url)
    }

    /// 使用默认参数创建新的 App 客户端
    pub fn new(base_url: impl Into<String>) -> Result<Self, TransportError> {
        Self::builder(base_url).build()
    }

    /// 获取完整 URL
    pub fn full_url(&self, path: &str) -> String {
        join_base_and_path(&self.base_url, path)
    }

    /// 统一请求构建入口
    pub fn request(&self, method: HttpMethod, path: &str) -> RequestBuilder<'_> {
        let url = self.full_url(path);
        let request = match method {
            HttpMethod::Get => self.http.get(url),
            HttpMethod::Post => self.http.post(url),
        };

        RequestBuilder {
            client: self,
            method,
            path: path.to_string(),
            request,
            headers: None,
            token: None,
        }
    }

    /// 处理响应
    async fn handle_response(
        &self,
        response: reqwest::Response,
    ) -> Result<reqwest::Response, TransportError> {
        let status = response.status();
        if status.is_success() {
            debug!(
                step = "transport.response.ok",
                status = status.as_u16(),
                "http status success"
            );
            Ok(response)
        } else {
            let body = response.text().await.unwrap_or_default();
            warn!(
                step = "transport.response.error_status",
                status = status.as_u16(),
                body_len = body.len(),
                "http status not success"
            );
            Err(TransportError::HttpStatus {
                status: status.as_u16(),
                body,
            })
        }
    }

    /// 解析 JSON 响应
    pub async fn parse_json<T: for<'de> serde::Deserialize<'de>>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, TransportError> {
        let text = response.text().await?;
        debug!(
            step = "transport.parse_json",
            payload_len = text.len(),
            "parsing json response"
        );
        let parsed = serde_json::from_str(&text).map_err(TransportError::from);
        if let Err(err) = &parsed {
            error!(
                step = "transport.parse_json.error",
                err = %err,
                "json parse failed"
            );
        }
        parsed
    }

    /// 解析并校验通用业务响应包裹，返回其中的 data。
    pub async fn parse_biz_json<T: for<'de> serde::Deserialize<'de>>(
        &self,
        response: reqwest::Response,
        service: &'static str,
    ) -> Result<T, AppError> {
        debug!(
            step = "transport.parse_biz_json",
            service,
            "parsing business envelope"
        );
        let envelope = self.parse_json::<BizEnvelope<T>>(response).await?;
        Ok(envelope.into_data(service)?)
    }
}
