use crate::constants::dorm;
use crate::constants::endpoints::DORM_LIST;
use crate::error::{AppError, ServiceError};
use crate::models::dorm::DormListData;
use crate::transport::{AppClient, HttpMethod};
use crate::utils::sign::build_app_signed_headers;

#[derive(Debug, serde::Serialize)]
pub struct DormListRequest {
    #[serde(rename = "current")]
    pub current: i32,
    #[serde(rename = "size")]
    pub size: i32,
}

impl DormListRequest {
    pub fn new(current: i32, size: i32) -> Self {
        Self { current, size }
    }
}

impl Default for DormListRequest {
    fn default() -> Self {
        Self {
            current: dorm::LIST_CURRENT,
            size: dorm::LIST_SIZE,
        }
    }
}

/// 宿舍签到任务列表服务
pub struct DormListService<'a> {
    client: &'a AppClient,
}

impl<'a> DormListService<'a> {
    pub fn new(client: &'a AppClient) -> Self {
        Self { client }
    }

    /// 获取宿舍签到任务列表（仅返回领域数据，不暴露网络响应包裹结构）
    pub async fn list(
        &self,
        token: &str,
        current: i32,
        size: i32,
    ) -> Result<DormListData, AppError> {
        let request = DormListRequest::new(current, size);
        let headers = build_app_signed_headers(&self.client.full_url(DORM_LIST), token)
            .map_err(|msg| ServiceError::BuildError {
                service: "dorm.list",
                msg,
            })?;
        let response = self
            .client
            .request(HttpMethod::Get, DORM_LIST)
            .header_map(headers)
            .query(&request)?
            .send()
            .await?;

        let envelope = self
            .client
            .parse_json::<DormListEnvelopeResponse>(response)
            .await?;

        if !envelope.success || envelope.code != 200 {
            return Err(ServiceError::RemoteBusiness {
                service: "dorm.list",
                code: envelope.code,
                msg: envelope.msg,
            }
            .into());
        }

        Ok(envelope.data)
    }

    /// 使用默认分页参数获取宿舍签到任务列表（current=1,size=15）
    pub async fn list_with_default(&self, token: &str) -> Result<DormListData, AppError> {
        let request = DormListRequest::default();
        self.list(token, request.current, request.size).await
    }
}

/// 仅在服务层使用的网络响应包裹
#[derive(Debug, serde::Deserialize)]
struct DormListEnvelopeResponse {
    code: i32,
    success: bool,
    data: DormListData,
    msg: String,
}
