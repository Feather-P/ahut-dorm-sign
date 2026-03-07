use crate::constants::client::{DEFAULT_TIMEOUT_SECS, DEFAULT_USER_AGENT};
use crate::error::TransportError;
use crate::utils::sign::build_app_signed_headers;
use crate::utils::url::join_base_and_path;
use reqwest::{Client, header::HeaderMap};
use serde_json::Value;
use std::time::Duration;

/// App 客户端构建器
#[derive(Debug, Clone)]
pub struct AppClientBuilder {
    base_url: String,
    timeout: Duration,
    user_agent: String,
    enable_logging: bool,
}

impl AppClientBuilder {
    /// 创建构建器
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            user_agent: DEFAULT_USER_AGENT.to_string(),
            enable_logging: false,
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

    /// 启用日志
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
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
            enable_logging: self.enable_logging,
        })
    }
}

/// App 客户端
#[derive(Clone)]
pub struct AppClient {
    pub(crate) http: Client,
    base_url: String,
    enable_logging: bool,
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
    headers: Option<HeaderMap>,
    query: Option<Value>,
    json_body: Option<Value>,
    token: Option<String>,
}

impl<'a> RequestBuilder<'a> {
    pub fn header_map(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    pub fn query<T: serde::Serialize>(mut self, query: &T) -> Result<Self, TransportError> {
        self.query = Some(
            serde_json::to_value(query).map_err(|e| TransportError::Serialize {
                field: "query",
                source: e,
            })?,
        );
        Ok(self)
    }

    pub fn sign(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    pub fn json<T: serde::Serialize>(mut self, body: &T) -> Result<Self, TransportError> {
        self.json_body =
            Some(
                serde_json::to_value(body).map_err(|e| TransportError::Serialize {
                    field: "json_body",
                    source: e,
                })?,
            );
        Ok(self)
    }

    pub async fn send(self) -> Result<reqwest::Response, TransportError> {
        let url = self.client.full_url(&self.path);
        let method = match self.method {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
        };
        self.client.log_request(method, &url);

        let mut request = match self.method {
            HttpMethod::Get => self.client.http.get(&url),
            HttpMethod::Post => self.client.http.post(&url),
        };

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

        if let Some(query) = self.query {
            request = request.query(&query);
        }

        if let Some(json_body) = self.json_body {
            request = request.json(&json_body);
        }

        let response = request.send().await?;
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
        RequestBuilder {
            client: self,
            method,
            path: path.to_string(),
            headers: None,
            query: None,
            json_body: None,
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
            Ok(response)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(TransportError::HttpStatus {
                status: status.as_u16(),
                body,
            })
        }
    }

    /// 日志记录请求
    fn log_request(&self, method: &str, url: &str) {
        if self.enable_logging {
            eprintln!("[HTTP] {} {}", method, url);
        }
    }

    /// 解析 JSON 响应
    pub async fn parse_json<T: for<'de> serde::Deserialize<'de>>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, TransportError> {
        let text = response.text().await?;
        serde_json::from_str(&text).map_err(TransportError::from)
    }
}
