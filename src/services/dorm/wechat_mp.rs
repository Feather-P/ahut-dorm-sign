use std::sync::Arc;

use crate::{
    AppClient, AppError, WechatMpConfigService,
    constants::wechat_mp::DORM_SIGN_CONFIG_URL_BASE,
};
use tracing::{error, info, instrument};

pub struct DormWechatMpService {
    wechat_service: WechatMpConfigService,
}

impl DormWechatMpService {
    pub fn new(client: Arc<AppClient>) -> Self {
        Self {
            wechat_service: WechatMpConfigService::new(client),
        }
    }

    #[instrument(
        name = "service.dorm_wechat_mp_send",
        skip(self, token),
        fields(step = "dorm.wechat_mp", task_id = task_id, student_id = student_id)
    )]
    pub async fn dorm_wechat_mp_send(
        &self,
        task_id: &str,
        student_id: &str,
        token: &str,
    ) -> Result<(), AppError> {
        let config_url = format!(
            "{DORM_SIGN_CONFIG_URL_BASE}?taskId={task_id}&autoSign=1&scanSign=0&userId={student_id}"
        );

        info!(
            step = "dorm.wechat_mp.request",
            task_id,
            student_id,
            "sending dorm wechat mp request"
        );
        let result = self.wechat_service.wechat_check(token, config_url).await;
        if let Err(err) = &result {
            error!(
                step = "dorm.wechat_mp.result",
                branch = "error",
                err = %err,
                "dorm wechat mp failed"
            );
        } else {
            info!(
                step = "dorm.wechat_mp.result",
                branch = "success",
                "dorm wechat mp success"
            );
        }
        result
    }
}
