use crate::{
    AppClient, AppError, ServiceError, constants::endpoints::DORM_WECHAT_MP_CONFIG,
    transport::HttpMethod,
};
use derive_builder::Builder;
use serde::Serialize;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

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

pub struct WechatMpConfigService {
    client: Arc<AppClient>,
}

impl WechatMpConfigService {
    pub fn new(client: Arc<AppClient>) -> Self {
        Self { client }
    }

    #[instrument(
        name = "service.wechat_mp_check",
        skip(self, token, config_url),
        fields(step = "wechat_mp.check")
    )]
    pub async fn wechat_check(
        &self,
        token: &str,
        config_url: impl ToString,
    ) -> Result<(), AppError> {
        info!(
            step = "wechat_mp.request.prepare",
            method = "GET",
            path = DORM_WECHAT_MP_CONFIG,
            "preparing wechat mp check request"
        );
        let request = WechatMpConfigRequestBuilder::default()
            .config_url(config_url.to_string())
            .build()
            .map_err(ServiceError::from)?;

        debug!(
            step = "wechat_mp.request.send",
            method = "GET",
            path = DORM_WECHAT_MP_CONFIG,
            "sending wechat mp config request"
        );
        let response = self
            .client
            .request(HttpMethod::Get, DORM_WECHAT_MP_CONFIG)
            .sign(token)
            .query(&request)?
            .send()
            .await?;

        debug!(
            step = "wechat_mp.response.received",
            status = response.status().as_u16(),
            "wechat mp response received"
        );

        let parsed = self
            .client
            .parse_biz_json::<serde_json::Value>(response, "wechat_mp.wechat_check")
            .await
            .map(|_| ());

        match &parsed {
            Ok(_) => info!(
                step = "wechat_mp.result",
                branch = "success",
                "wechat mp check success"
            ),
            Err(err) => error!(
                step = "wechat_mp.result",
                branch = "error",
                err = %err,
                "wechat mp check failed"
            ),
        }

        parsed
    }
}
