use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{
    error::DomainError,
    school::{
        session::SchoolSession, sign_config::SchoolSignConfig, task::SchoolSignTask,
        user::SchoolUser,
    },
};

type SchoolRepositoryResult<T> = Result<T, DomainError>;

#[async_trait]
pub trait SchoolUserRepository: Send + Sync {
    async fn find_by_owner_and_student(
        &self,
        student_id: &str,
        owner_user_id: uuid::Uuid,
    ) -> SchoolRepositoryResult<Option<SchoolUser>>;

    async fn list_by_owner_user_id(
        &self,
        owner_user_id: uuid::Uuid,
    ) -> SchoolRepositoryResult<Vec<SchoolUser>>;

    async fn save(&self, user: SchoolUser) -> SchoolRepositoryResult<()>;

    async fn delete_by_student_id(&self, student_id: &str) -> SchoolRepositoryResult<bool>;
}

#[async_trait]
pub trait SchoolSignConfigRepository: Send + Sync {
    async fn find_enabled_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> SchoolRepositoryResult<Option<SchoolSignConfig>>;

    async fn find_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> SchoolRepositoryResult<Option<SchoolSignConfig>>;

    async fn list_by_student_id(
        &self,
        student_id: &str,
    ) -> SchoolRepositoryResult<Vec<SchoolSignConfig>>;

    async fn list_enabled_by_student_id(
        &self,
        student_id: &str,
    ) -> SchoolRepositoryResult<Vec<SchoolSignConfig>>;

    async fn save(&self, config: SchoolSignConfig) -> SchoolRepositoryResult<()>;

    async fn delete_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> SchoolRepositoryResult<bool>;

    async fn set_enabled(
        &self,
        student_id: &str,
        school_task_id: &str,
        enabled: bool,
    ) -> SchoolRepositoryResult<bool>;
}

#[async_trait]
pub trait SchoolSignTaskRepository: Send + Sync {
    async fn find_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> SchoolRepositoryResult<Option<SchoolSignTask>>;
    async fn find_runnable(
        &self,
        student_id: &str,
        utc_now: DateTime<Utc>,
    ) -> SchoolRepositoryResult<Option<SchoolSignTask>>;

    async fn list_by_student_id(
        &self,
        student_id: &str,
    ) -> SchoolRepositoryResult<Vec<SchoolSignTask>>;

    async fn save(&self, sign_task: SchoolSignTask) -> SchoolRepositoryResult<()>;

    async fn delete_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> SchoolRepositoryResult<bool>;
}

#[async_trait]
pub trait SchoolSessionRepository: Send + Sync {
    async fn find_by_owner_and_student(
        &self,
        owner_user_id: uuid::Uuid,
        student_id: &str,
    ) -> SchoolRepositoryResult<Option<SchoolSession>>;

    async fn save(&self, session: SchoolSession) -> SchoolRepositoryResult<()>;

    async fn delete_by_owner_and_student(
        &self,
        owner_user_id: uuid::Uuid,
        student_id: &str,
    ) -> SchoolRepositoryResult<bool>;
}
