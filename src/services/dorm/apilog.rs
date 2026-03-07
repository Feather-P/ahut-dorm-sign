use std::sync::Arc;

use crate::{
    AppClient, AppError, constants::apilog::DORM_API_LOG_MENU_TITLE,
    services::apilog::ApiLogService,
};

pub struct DormApiLogService {
    api_logger: ApiLogService,
}

impl DormApiLogService {
    pub fn new(client: Arc<AppClient>) -> Self {
        Self {
            api_logger: ApiLogService::new(client),
        }
    }

    pub async fn dorm_api_log(&self, token: &str) -> Result<(), AppError> {
        self.api_logger.api_log(token, DORM_API_LOG_MENU_TITLE).await
    }
}
