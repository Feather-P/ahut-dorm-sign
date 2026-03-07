use crate::constants::dorm;
use crate::constants::endpoints::DORM_LIST;
use crate::error::AppError;
use crate::models::dorm::DormListData;
use crate::transport::{AppClient, HttpMethod};
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

#[derive(Debug, serde::Serialize)]
pub struct DormListRequest {
    #[serde(rename = "current")]
    pub current: i32,
    #[serde(rename = "size")]
    pub size: i32,
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
pub struct DormListService {
    client: Arc<AppClient>,
}

impl DormListService {
    pub fn new(client: Arc<AppClient>) -> Self {
        Self { client }
    }

    /// 获取宿舍签到任务列表（仅返回领域数据，不暴露网络响应包裹结构）
    #[instrument(
        name = "service.dorm_list",
        skip(self, token),
        fields(step = "dorm.list", current = current, size = size)
    )]
    pub async fn list(
        &self,
        token: &str,
        current: i32,
        size: i32,
    ) -> Result<DormListData, AppError> {
        info!(
            step = "dorm.list.request",
            method = "GET",
            path = DORM_LIST,
            "sending dorm list request"
        );
        let request = DormListRequest { current, size };
        let response = self
            .client
            .request(HttpMethod::Get, DORM_LIST)
            .sign(token)
            .query(&request)?
            .send()
            .await?;

        debug!(
            step = "dorm.list.response.received",
            status = response.status().as_u16(),
            "dorm list http response received"
        );

        let parsed: Result<DormListData, AppError> =
            self.client.parse_biz_json(response, "dorm.list").await;
        match &parsed {
            Ok(data) => info!(
                step = "dorm.list.result",
                branch = "success",
                record_count = data.records.len(),
                "dorm list parsed"
            ),
            Err(err) => error!(
                step = "dorm.list.result",
                branch = "error",
                err = %err,
                "dorm list failed"
            ),
        }
        parsed
    }

    /// 使用默认分页参数获取宿舍签到任务列表（current=1,size=15）
    pub async fn list_with_default(&self, token: &str) -> Result<DormListData, AppError> {
        let request = DormListRequest::default();
        self.list(token, request.current, request.size).await
    }
}
