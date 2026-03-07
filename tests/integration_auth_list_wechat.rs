use ahut_drom_sign::{
    AppClient, BASE_URL, DormListService, LoginService, WechatMpConfigService,
};

fn read_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("缺少环境变量: {name}"))
}

/// 集成测试：登录 -> 获取 token -> 调用宿舍任务列表接口 -> 调用微信 mp 配置记录接口
///
/// 运行方式：
/// AHUT_USERNAME=你的学号 AHUT_PASSWORD=你的密码 cargo test --test integration_auth_list_wechat -- --ignored --nocapture
#[tokio::test]
#[ignore = "需要真实账号与网络环境，默认跳过"]
async fn it_login_then_test_list_and_wechat_mp() {
    let username = read_env("AHUT_USERNAME");
    let password = read_env("AHUT_PASSWORD");

    let client = AppClient::builder(BASE_URL)
        .with_logging(true)
        .build()
        .expect("构建客户端失败");

    let login_service = LoginService::new(&client);
    let auth = login_service
        .login_with_default(&username, &password)
        .await
        .expect("登录失败");

    let token = auth.access_token;

    let list_service = DormListService::new(&client);
    let list_data = list_service
        .list_with_default(&token)
        .await
        .expect("调用 dorm list 失败");

    // 至少确保请求成功且返回结构可解析
    let _ = list_data;

    let wechat_service = WechatMpConfigService::new(&client);
    wechat_service
        .wechat_check(&token, BASE_URL)
        .await
        .expect("调用 wechat_mp 失败");
}

