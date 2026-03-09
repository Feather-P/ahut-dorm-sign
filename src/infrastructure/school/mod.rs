pub mod week_mapper;

use async_trait::async_trait;

use crate::domain::school::{
    config::SchoolSignConfig,
    repository::{SchoolRepositoryResult, SchoolSignConfigRepository, SchoolUserRepository},
    user::SchoolUser,
};

/// 最小占位实现：用于在未接入持久化前满足仓储依赖。
///
/// 语义：
/// - 查询始终返回 `None`
/// - 保存始终返回 `Ok(())`
pub struct NullSchoolRepository;

#[async_trait]
impl SchoolUserRepository for NullSchoolRepository {
    async fn find_by_student_id(&self, _student_id: &str) -> SchoolRepositoryResult<Option<SchoolUser>> {
        Ok(None)
    }

    async fn save(&self, _user: SchoolUser) -> SchoolRepositoryResult<()> {
        Ok(())
    }
}

#[async_trait]
impl SchoolSignConfigRepository for NullSchoolRepository {
    async fn find_enabled_by_student_and_task(
        &self,
        _student_id: &str,
        _school_task_id: &str,
    ) -> SchoolRepositoryResult<Option<SchoolSignConfig>> {
        Ok(None)
    }

    async fn save(&self, _config: SchoolSignConfig) -> SchoolRepositoryResult<()> {
        Ok(())
    }
}
