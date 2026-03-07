use ahut_dorm_sign::{
    AppClient, BASE_URL, DormApiLogService, DormListService, DormWechatMpService, LoginService,
};
use std::sync::Arc;

fn read_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("缺少环境变量: {name}"))
}

/// 集成测试：登录 -> 获取 token -> 调用宿舍任务列表接口 -> 调用宿舍微信 mp 接口 -> 调用页面访问日志接口
///
/// 运行方式：
/// AHUT_USERNAME=你的学号 AHUT_PASSWORD=你的密码 cargo test --test integration_all_sequence -- --ignored --nocapture
#[tokio::test]
#[ignore = "需要真实账号与网络环境，默认跳过"]
async fn it_login_then_test_dorm_features_in_sequence() {
    let username = read_env("AHUT_USERNAME");
    let password = read_env("AHUT_PASSWORD");

    let client = Arc::new(
        AppClient::builder(BASE_URL)
        .with_logging(true)
        .build()
        .expect("构建客户端失败"),
    );

    let login_service = LoginService::new(Arc::clone(&client));
    let auth = login_service
        .login_with_default(&username, &password)
        .await
        .expect("登录失败");

    let token = auth.access_token;

    let list_service = DormListService::new(Arc::clone(&client));
    let list_data = list_service
        .list_with_default(&token)
        .await
        .expect("调用 dorm list 失败");

    // 至少确保请求成功且返回结构可解析
    let first_task = list_data
        .records
        .first()
        .expect("dorm list 为空，无法继续测试 dorm_wechat_mp/apilog 顺序调用");

    let wechat_service = DormWechatMpService::new(Arc::clone(&client));
    wechat_service
        .dorm_wechat_mp_send(&first_task.task_id, &username, &token)
        .await
        .expect("调用 dorm wechat_mp 失败");

    let api_log_service = DormApiLogService::new(Arc::clone(&client));
    api_log_service
        .dorm_api_log(&token)
        .await
        .expect("调用 dorm apilog 失败");
}
