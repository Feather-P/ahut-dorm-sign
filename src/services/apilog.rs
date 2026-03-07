use crate::{
    AppClient, AppError, ServiceError, constants::{endpoints::{DORM_API_LOG_SAVE}},
    transport::HttpMethod,
};
use derive_builder::Builder;
use serde::Serialize;
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Debug, Serialize, Builder)]
pub struct ApiLogRequest {
    #[serde(rename = "menuTitle")]
    menu_title: String,
}

impl From<ApiLogRequestBuilderError> for ServiceError {
    fn from(err: ApiLogRequestBuilderError) -> Self {
        ServiceError::BuildError {
            service: "api_log",
            msg: err.to_string(),
        }
    }
}

pub struct ApiLogService {
    client: Arc<AppClient>,
}

impl ApiLogService {
    pub fn new(client: Arc<AppClient>) -> Self {
        Self { client }
    }

    #[instrument(
        name = "service.api_log",
        skip(self, token, menu_title),
        fields(step = "apilog.save")
    )]
    pub async fn api_log(
        &self,
        token: &str,
        menu_title: impl ToString,
    ) -> Result<(), AppError> {
        let menu_title = menu_title.to_string();
        info!(
            step = "apilog.request.prepare",
            method = "POST",
            path = DORM_API_LOG_SAVE,
            menu_title = %menu_title,
            "preparing api log request"
        );
        let request = ApiLogRequestBuilder::default()
            .menu_title(menu_title)
            .build()
            .map_err(ServiceError::from)?;
        // 仅要求 HTTP 状态码成功即可：
        // RequestBuilder::send() 内部已通过 handle_response 做 2xx 校验，
        let result = self
            .client
            .request(HttpMethod::Post, DORM_API_LOG_SAVE)
            .sign(token)
            .query(&request)
            .send()
            .await?;

        info!(
            step = "apilog.response.received",
            status = result.status().as_u16(),
            branch = "success",
            "api log http response received"
        );

        Ok(())
    }
}
