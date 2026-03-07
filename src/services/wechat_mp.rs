use crate::{
    AppClient, AppError, ServiceError, constants::endpoints::DORM_WECHAT_MP_CONFIG,
    transport::HttpMethod,
};
use derive_builder::Builder;
use serde::Serialize;

#[derive(Debug, Serialize, Builder)]
pub struct WechatMpConfigRequest {
    #[serde(rename = "configUrl")]
    config_url: String,
}

impl From<WechatMpConfigRequestBuilderError> for ServiceError {
    fn from(err: WechatMpConfigRequestBuilderError) -> Self {
        ServiceError::BuildError {
            service: "wechat_mp",
            msg: err.to_string(),
        }
    }
}

pub struct WechatMpConfigService<'a> {
    client: &'a AppClient,
}

impl<'a> WechatMpConfigService<'a> {
    pub fn new(client: &'a AppClient) -> Self {
        Self { client }
    }

    pub async fn wechat_check(
        &self,
        token: &str,
        config_url: impl ToString,
    ) -> Result<(), AppError> {
        let request = WechatMpConfigRequestBuilder::default()
            .config_url(config_url.to_string())
            .build()
            .map_err(ServiceError::from)?;
        let response = self
            .client
            .request(HttpMethod::Get, DORM_WECHAT_MP_CONFIG)
            .sign(token)
            .query(&request)?
            .send()
            .await?;

        self.client
            .parse_biz_json::<serde_json::Value>(response, "wechat_mp.wechat_check")
            .await
            .map(|_| ())
    }
}
