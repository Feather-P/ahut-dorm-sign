use std::sync::Arc;

use crate::{
    AppClient, AppError, WechatMpConfigService,
    constants::wechat_mp::DORM_SIGN_CONFIG_URL_BASE,
};

pub struct DormWechatMpService {
    wechat_service: WechatMpConfigService,
}

impl DormWechatMpService {
    pub fn new(client: Arc<AppClient>) -> Self {
        Self {
            wechat_service: WechatMpConfigService::new(client),
        }
    }

    pub async fn dorm_wechat_mp_send(
        &self,
        task_id: &str,
        student_id: &str,
        token: &str,
    ) -> Result<(), AppError> {
        let config_url = format!(
            "{DORM_SIGN_CONFIG_URL_BASE}?taskId={task_id}&autoSign=1&scanSign=0&userId={student_id}"
        );

        self.wechat_service.wechat_check(token, config_url).await
    }
}
