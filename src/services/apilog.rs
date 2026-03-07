use crate::{
    AppClient, AppError, ServiceError, constants::{endpoints::{DORM_API_LOG_SAVE}},
    transport::HttpMethod,
};
use derive_builder::Builder;
use serde::Serialize;
use std::sync::Arc;

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

    pub async fn api_log(
        &self,
        token: &str,
        menu_title: impl ToString,
    ) -> Result<(), AppError> {
        let request = ApiLogRequestBuilder::default()
            .menu_title(menu_title.to_string())
            .build()
            .map_err(ServiceError::from)?;
        // 仅要求 HTTP 状态码成功即可：
        // RequestBuilder::send() 内部已通过 handle_response 做 2xx 校验，
        self
            .client
            .request(HttpMethod::Post, DORM_API_LOG_SAVE)
            .sign(token)
            .query(&request)?
            .send()
            .await?;

        Ok(())
    }
}
