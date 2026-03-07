use std::sync::Arc;

use crate::{
    AppClient, AppError, constants::apilog::DORM_API_LOG_MENU_TITLE,
    services::apilog::ApiLogService,
};
use tracing::{error, info, instrument};

pub struct DormApiLogService {
    api_logger: ApiLogService,
}

impl DormApiLogService {
    pub fn new(client: Arc<AppClient>) -> Self {
        Self {
            api_logger: ApiLogService::new(client),
        }
    }

    #[instrument(
        name = "service.dorm_api_log",
        skip(self, token),
        fields(step = "dorm.apilog")
    )]
    pub async fn dorm_api_log(&self, token: &str) -> Result<(), AppError> {
        info!(
            step = "dorm.apilog.request",
            menu_title = DORM_API_LOG_MENU_TITLE,
            "sending dorm api log"
        );
        let result = self.api_logger.api_log(token, DORM_API_LOG_MENU_TITLE).await;
        if let Err(err) = &result {
            error!(
                step = "dorm.apilog.result",
                branch = "error",
                err = %err,
                "dorm api log failed"
            );
        } else {
            info!(
                step = "dorm.apilog.result",
                branch = "success",
                "dorm api log success"
            );
        }
        result
    }
}
