use std::{env, path::PathBuf};

use ahut_dorm_sign::domain::{
    error::DomainError,
    school::{
        checkin_flow::{ExecuteCheckinInput, SchoolCheckinFlowService},
        gateway::SchoolGateway,
        policy::{SchoolAuthDecider, SchoolAuthPolicyConfig},
        repository::{SchoolSessionRepository, SchoolSignTaskRepository, SchoolUserRepository},
        session::SchoolSession,
        task::SchoolSignTask,
        user::SchoolUser,
    },
};
use ahut_dorm_sign::infrastructure::config::school::SchoolInfraConfig;
use ahut_dorm_sign::infrastructure::school::{
    gateway::AhutGateway,
    security::{AesGcmSchoolCredentialProtector, FlySourceSigner},
};
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use dotenvy::from_path;
use tokio::sync::Mutex;
use uuid::Uuid;

/// 仅用于本地联调的真实链路测试：
/// 1. 从指定 dotenv 文件和环境变量读取测试凭据/定位信息
/// 2. 通过网关先拉取远端活动任务列表，选择一个任务作为签到目标
/// 3. 构造最小 mock repositories（仅承载 Flow 运行所需数据）
/// 4. 通过 CheckinFlowService 执行完整签到流程（认证/会话/prepare/submit）
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

    let credential_protector =
        AesGcmSchoolCredentialProtector::new(infra_config.security.master_key)
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

    let custom_uas = env::var("TEST_CUSTOM_USER_AGENT_POOL")
        .ok()
        .map(|s| {
            s.split(',')
                .map(|it| it.trim().to_string())
                .filter(|it| !it.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let gateway = AhutGateway::new(
        client,
        base_url,
        school_fixed_authorization,
        Box::new(credential_protector),
        Box::new(FlySourceSigner),
    );

    let point = ahut_dorm_sign::domain::school::location::GeoPoint::new(sign_lng, sign_lat)
        .context("定位坐标非法")?;

    let now = Utc::now();
    let prefetch_ua = gateway_config.pick_user_agent(&custom_uas);
    let token = gateway
        .authenticate(&school_user)
        .await
        .context("预取任务前认证失败")?;
    let prefetch_session =
        SchoolSession::new(Uuid::nil(), student_id.clone(), token).context("构建预取会话失败")?;
    let active_tasks = gateway
        .fetch_active_task_list(&prefetch_session, &prefetch_ua)
        .await
        .context("预取任务列表失败")?;
    let selected_task = active_tasks
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("远端任务列表为空"))?;
    let school_task_id = selected_task.school_task_id().to_string();

    let mock_user_repo = MockSchoolUserRepository {
        user: Some(school_user),
    };
    let mock_session_repo = MockSchoolSessionRepository {
        session: Mutex::new(None),
    };
    let mock_task_repo = MockSchoolSignTaskRepository {
        tasks: Mutex::new(vec![StoredTask::from_task(&selected_task)]),
    };
    let auth_decider = SchoolAuthDecider::new(
        SchoolAuthPolicyConfig::new(Duration::seconds(300), 1).context("构建认证策略失败")?,
    );

    let flow = SchoolCheckinFlowService::new(
        &gateway,
        &mock_user_repo,
        &mock_session_repo,
        &mock_task_repo,
        &auth_decider,
    );

    flow.execute(ExecuteCheckinInput {
        owner_user_id: Uuid::nil(),
        student_id: student_id.clone(),
        school_task_id: school_task_id.clone(),
        user_agent: prefetch_ua,
        point,
        accuracy_meters: sign_accuracy_meters,
        utc_now: now,
    })
    .await
    .with_context(|| format!("FlowService 执行签到失败: task_id={school_task_id}"))?;

    Ok(())
}

struct MockSchoolUserRepository {
    user: Option<SchoolUser>,
}

#[async_trait]
impl SchoolUserRepository for MockSchoolUserRepository {
    async fn find_by_owner_and_student(
        &self,
        student_id: &str,
        owner_user_id: Uuid,
    ) -> Result<Option<SchoolUser>, DomainError> {
        Ok(self.user.as_ref().and_then(|u| {
            if u.student_id() == student_id && *u.owner_user_id() == owner_user_id {
                Some(
                    SchoolUser::new(
                        u.student_id().to_string(),
                        *u.owner_user_id(),
                        u.user_name().to_string(),
                        ahut_dorm_sign::domain::school::credential::SchoolCredential::from_storage(
                            &u.credential_storage(),
                        )
                        .ok()?,
                    )
                    .ok()?,
                )
            } else {
                None
            }
        }))
    }

    async fn list_by_owner_user_id(
        &self,
        _owner_user_id: Uuid,
    ) -> Result<Vec<SchoolUser>, DomainError> {
        Ok(vec![])
    }

    async fn save(&self, _user: SchoolUser) -> Result<(), DomainError> {
        Ok(())
    }

    async fn delete_by_owner_and_student(
        &self,
        _owner_user_id: Uuid,
        _student_id: &str,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
}

struct MockSchoolSessionRepository {
    session: Mutex<Option<SchoolSession>>,
}

#[async_trait]
impl SchoolSessionRepository for MockSchoolSessionRepository {
    async fn find_by_owner_and_student(
        &self,
        owner_user_id: Uuid,
        student_id: &str,
    ) -> Result<Option<SchoolSession>, DomainError> {
        let guard = self.session.lock().await;
        if let Some(s) = guard.as_ref() {
            if *s.owner_user_id() == owner_user_id && s.student_id() == student_id {
                let token = ahut_dorm_sign::domain::school::token::SchoolToken::new(
                    s.access_token().to_string(),
                    s.refresh_token().to_string(),
                    s.expired_at(),
                )?;
                return Ok(Some(SchoolSession::new(
                    *s.owner_user_id(),
                    s.student_id().to_string(),
                    token,
                )?));
            }
        }
        Ok(None)
    }

    async fn save(&self, session: SchoolSession) -> Result<(), DomainError> {
        let mut guard = self.session.lock().await;
        *guard = Some(session);
        Ok(())
    }

    async fn delete_by_owner_and_student(
        &self,
        _owner_user_id: Uuid,
        _student_id: &str,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
}

struct MockSchoolSignTaskRepository {
    tasks: Mutex<Vec<StoredTask>>,
}

struct StoredTask {
    id: Uuid,
    student_id: String,
    school_task_id: String,
    title: String,
    date_range: ahut_dorm_sign::domain::school::task::DateRange,
    daily_time_window: ahut_dorm_sign::domain::school::task::TimeWindow,
    days_of_week: chrono::WeekdaySet,
    time_zone: chrono_tz::Tz,
}

impl StoredTask {
    fn from_task(task: &SchoolSignTask) -> Self {
        Self {
            id: *task.id(),
            student_id: task.student_id().to_string(),
            school_task_id: task.school_task_id().to_string(),
            title: task.title().to_string(),
            date_range: *task.date_range(),
            daily_time_window: *task.daily_time_window(),
            days_of_week: *task.days_of_week(),
            time_zone: *task.time_zone(),
        }
    }
}

#[async_trait]
impl SchoolSignTaskRepository for MockSchoolSignTaskRepository {
    async fn find_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> Result<Option<SchoolSignTask>, DomainError> {
        let guard = self.tasks.lock().await;
        Ok(guard.iter().find_map(|t| {
            if t.student_id == student_id && t.school_task_id == school_task_id {
                SchoolSignTask::new(
                    t.id,
                    t.student_id.clone(),
                    t.school_task_id.clone(),
                    t.title.clone(),
                    t.date_range,
                    t.daily_time_window,
                    t.days_of_week,
                    t.time_zone,
                )
                .ok()
            } else {
                None
            }
        }))
    }

    async fn find_runnable(
        &self,
        _student_id: &str,
        _utc_now: DateTime<Utc>,
    ) -> Result<Option<SchoolSignTask>, DomainError> {
        Ok(None)
    }

    async fn list_by_student_id(
        &self,
        _student_id: &str,
    ) -> Result<Vec<SchoolSignTask>, DomainError> {
        Ok(vec![])
    }

    async fn save(&self, _sign_task: SchoolSignTask) -> Result<(), DomainError> {
        let mut guard = self.tasks.lock().await;
        guard.push(StoredTask::from_task(&_sign_task));
        Ok(())
    }

    async fn delete_by_student_and_task(
        &self,
        _student_id: &str,
        _school_task_id: &str,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
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
