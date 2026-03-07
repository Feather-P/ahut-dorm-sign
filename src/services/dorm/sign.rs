use std::sync::Arc;

use derive_builder::Builder;
use serde::Serialize;

use crate::{
    AppClient, AppError, ServiceError, constants::endpoints::DORM_ADD_RECORD,
    transport::HttpMethod, utils::random::random_with_range,
};

#[derive(Debug, Serialize, Builder)]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct DormSignRequest {
    pub task_id: String,
    #[builder(default)]
    pub sign_address: String,
    pub location_accuracy: f64,
    pub sign_lat: f64,
    pub sign_lng: f64,
    #[builder(default = "0")]
    pub sign_type: i32,
    #[builder(default)]
    pub file_id: String,
    #[builder(default = "\"/static/images/dormitory/photo.png\".to_string()")]
    pub img_base64: String,
    pub sign_date: String,
    pub sign_time: String,
    pub sign_week: String,
    #[builder(default)]
    pub scan_code: String,
}

impl From<DormSignRequestBuilderError> for ServiceError {
    fn from(err: DormSignRequestBuilderError) -> Self {
        Self::BuildError {
            service: "dorm.sign.dorm_sign_request_builder",
            msg: err.to_string(),
        }
    }
}

pub struct DormSignService {
    client: Arc<AppClient>,
}

impl DormSignService {
    pub fn new(client: Arc<AppClient>) -> Self {
        Self { client }
    }

    pub async fn sign(
        &self,
        token: &str,
        task_id: &str,
        sign_lat: f64,
        sign_lng: f64,
        sign_date: &str,
        sign_time: &str,
        sign_week: &str,
    ) -> Result<(), AppError> {
        let request = DormSignRequestBuilder::default()
            .task_id(task_id)
            .location_accuracy(random_with_range(15, 40))
            .sign_lat(sign_lat)
            .sign_lng(sign_lng)
            .sign_date(sign_date)
            .sign_time(sign_time)
            .sign_week(sign_week)
            .build()
            .map_err(ServiceError::from)?;
        let response = self
            .client
            .request(HttpMethod::Post, DORM_ADD_RECORD)
            .sign(token)
            .json(&request)
            .send()
            .await?;

        let parsed = self
            .client
            .parse_biz_json::<serde_json::Value>(response, "dorm.sign")
            .await
            .map(|_| ());
        parsed
    }
}
