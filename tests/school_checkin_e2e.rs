use std::{env, path::PathBuf};

use ahut_dorm_sign::domain::school::{
    gateway::SchoolGateway, session::SchoolSession, task::CheckinCommand, user::SchoolUser,
};
use ahut_dorm_sign::infrastructure::school::{
    gateway::AhutGateway,
    security::{AesGcmSchoolCredentialProtector, FlySourceSigner},
};
use ahut_dorm_sign::infrastructure::config::school::SchoolInfraConfig;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use dotenvy::from_path;
use uuid::Uuid;

/// 仅用于本地联调的真实链路测试：
/// 1. 从指定 dotenv 文件和环境变量读取测试凭据/定位信息
/// 2. 登录并获取会话 token
/// 3. 拉取当前可用任务
/// 4. 预热签到上下文并提交签到
///
/// 执行方式（默认忽略，避免 CI 误调用真实接口）：
/// cargo test --test school_checkin_e2e test_school_checkin_e2e_from_env -- --ignored --nocapture
#[tokio::test]
#[ignore = "真实学校接口 E2E，仅本地手动执行"]
async fn test_school_checkin_e2e_from_env() -> Result<()> {
    let env_path = resolve_test_env_path();
    if let Some(path) = env_path {
        eprintln!("[test] loading dotenv from: {}", path.display());
        from_path(&path).with_context(|| format!("加载测试 dotenv 失败: {}", path.display()))?;
    } else {
        eprintln!("[test] no dotenv file found, fallback to process env only");
    }

    let student_id = required_env("TEST_SCHOOL_STUDENT_ID")?;
    let plain_password = required_env("TEST_SCHOOL_PASSWORD")?;
    let sign_lng = parse_env_f64("TEST_SIGN_LNG")?;
    let sign_lat = parse_env_f64("TEST_SIGN_LAT")?;
    let sign_accuracy_meters = parse_env_f64_or("TEST_SIGN_ACCURACY_METERS", 15.0)?;
    let school_fixed_authorization = required_env("SCHOOL_FIXED_AUTHORIZATION")?;

    let infra_config = SchoolInfraConfig::from_env().context("加载基础设施配置失败")?;
    let gateway_config = infra_config.gateway;
    let client = gateway_config
        .build_client()
        .context("构建 HTTP 客户端失败")?;
    let base_url = reqwest::Url::parse(&gateway_config.base_url)
        .with_context(|| format!("无效 AHUT_BASE_URL: {}", gateway_config.base_url))?;

    let credential_protector = AesGcmSchoolCredentialProtector::new(infra_config.security.master_key)
        .context("加载凭据加密器失败（检查 SCHOOL_CREDENTIAL_MASTER_KEY）")?;

    let school_password_md5 = format!("{:x}", md5::compute(plain_password));
    let school_user = SchoolUser::new(
        student_id.clone(),
        Uuid::nil(),
        student_id.clone(),
        ahut_dorm_sign::domain::school::crypto::SchoolCredentialProtector::encrypt(
            &credential_protector,
            &school_password_md5,
        ),
    )
    .context("构建 SchoolUser 失败")?;

    let gateway = AhutGateway::new(
        client,
        base_url,
        school_fixed_authorization,
        Box::new(credential_protector),
        Box::new(FlySourceSigner),
    );

    let token = gateway
        .authenticate(&school_user)
        .await
        .context("认证失败")?;

    let session = SchoolSession::new(Uuid::nil(), student_id.clone(), token).context("构建会话失败")?;

    let active_tasks = gateway
        .fetch_active_task_list(&session)
        .await
        .context("拉取任务列表失败")?;

    if active_tasks.is_empty() {
        return Err(anyhow!("远端未返回任何可用任务"));
    }

    let task = &active_tasks[0];
    if !task.is_runnable_at(Utc::now()) {
        return Err(anyhow!(
            "远端返回任务当前不在可签到时间段，按预期判定测试失败: task_id={} title={}",
            task.school_task_id(),
            task.title()
        ));
    }
    let task_id = task.school_task_id().to_string();

    gateway
        .prepare_checkin_context(&session, &task_id)
        .await
        .context("准备签到上下文失败")?;

    let point = ahut_dorm_sign::domain::school::location::GeoPoint::new(sign_lng, sign_lat)
        .context("定位坐标非法")?;
    let command = CheckinCommand::new(&task_id, &point, sign_accuracy_meters, Utc::now())
        .context("构建签到命令失败")?;

    gateway
        .submit_checkin(&session, command)
        .await
        .context("提交签到失败")?;

    Ok(())
}

fn resolve_test_env_path() -> Option<PathBuf> {
    if let Ok(custom) = env::var("TEST_DOTENV_PATH") {
        let custom = custom.trim();
        if !custom.is_empty() {
            let path = PathBuf::from(custom);
            if path.exists() {
                return Some(path);
            }
        }
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        root.join("tests/.env.test.local"),
        root.join("tests/.env.test"),
    ];

    candidates.into_iter().find(|p| p.exists())
}

fn required_env(key: &str) -> Result<String> {
    let value = env::var(key).with_context(|| format!("缺少环境变量: {key}"))?;
    if value.trim().is_empty() {
        return Err(anyhow!("环境变量为空: {key}"));
    }
    Ok(value)
}

fn parse_env_f64(key: &str) -> Result<f64> {
    required_env(key)?
        .parse::<f64>()
        .with_context(|| format!("环境变量解析失败（f64）: {key}"))
}

fn parse_env_f64_or(key: &str, default: f64) -> Result<f64> {
    match env::var(key) {
        Ok(v) if !v.trim().is_empty() => v
            .parse::<f64>()
            .with_context(|| format!("环境变量解析失败（f64）: {key}")),
        _ => Ok(default),
    }
}
