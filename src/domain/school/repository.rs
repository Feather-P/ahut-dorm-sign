use async_trait::async_trait;

use crate::domain::{
    error::DomainError,
    school::{config::SchoolSignConfig, user::SchoolUser},
};

pub type SchoolRepositoryResult<T> = Result<T, DomainError>;

/// School 子域最小用户仓储：仅保留当前业务必需的“按学号读取 + 保存”。
#[async_trait]
pub trait SchoolUserRepository: Send + Sync {
    async fn find_by_student_id(&self, student_id: &str)
        -> SchoolRepositoryResult<Option<SchoolUser>>;

    async fn save(&self, user: SchoolUser) -> SchoolRepositoryResult<()>;
}

/// School 子域最小签到配置仓储：仅保留当前业务必需的“读取启用配置 + 保存”。
#[async_trait]
pub trait SchoolSignConfigRepository: Send + Sync {
    async fn find_enabled_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> SchoolRepositoryResult<Option<SchoolSignConfig>>;

    async fn save(&self, config: SchoolSignConfig) -> SchoolRepositoryResult<()>;
}
