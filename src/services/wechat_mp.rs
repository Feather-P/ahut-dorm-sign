use crate::{
    AppClient, AppError, ServiceError, constants::endpoints::DORM_WECHAT_MP_CONFIG,
    transport::HttpMethod, utils::sign::build_app_signed_headers,
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
        let headers = build_app_signed_headers(&self.client.full_url(DORM_WECHAT_MP_CONFIG), token)
            .map_err(|msg| ServiceError::BuildError {
                service: "wechat_check.headers_build",
                msg,
            })?;
        let response = self
            .client
            .request(HttpMethod::Get, DORM_WECHAT_MP_CONFIG)
            .header_map(headers)
            .query(&request)?
            .send()
            .await?;

        let envelope = self
            .client
            .parse_json::<WechatMpEnvelopeResponse>(response)
            .await?;

        if !envelope.success || envelope.code != 200 {
            return Err(ServiceError::RemoteBusiness {
                service: "wechat_mp.wechat_check",
                code: envelope.code,
                msg: envelope.msg,
            }
            .into());
        }

        Ok(())
    }
}

/// 仅在服务层使用的网络响应包裹
#[derive(Debug, serde::Deserialize)]
struct WechatMpEnvelopeResponse {
    code: i32,
    success: bool,
    msg: String,
}
