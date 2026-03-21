use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{
    error::DomainError,
    school::{
        session::SchoolSession, sign_config::SchoolSignConfig, task::SchoolSignTask,
        task_run::{RunState, SchoolSignRun},
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

    async fn delete_by_owner_and_student(
        &self,
        owner_user_id: uuid::Uuid,
        student_id: &str,
    ) -> SchoolRepositoryResult<bool>;
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
pub trait SchoolSignRunRepository: Send + Sync {
    async fn save(&self, run: SchoolSignRun) -> SchoolRepositoryResult<()>;

    async fn find_by_id(&self, run_id: uuid::Uuid) -> SchoolRepositoryResult<Option<SchoolSignRun>>;

    async fn find_by_task_and_date(
        &self,
        task_id: uuid::Uuid,
        biz_date: chrono::NaiveDate,
    ) -> SchoolRepositoryResult<Option<SchoolSignRun>>;

    async fn list_due_waiting(
        &self,
        utc_now: DateTime<Utc>,
        limit: u32,
    ) -> SchoolRepositoryResult<Vec<SchoolSignRun>>;

    async fn list_stale_running(
        &self,
        running_before_utc: DateTime<Utc>,
        limit: u32,
    ) -> SchoolRepositoryResult<Vec<SchoolSignRun>>;

    /// CAS 状态迁移: 仅当当前状态等于 expected_state 时更新为 new_state。
    async fn compare_and_set_state(
        &self,
        run_id: uuid::Uuid,
        expected_state: RunState,
        new_state: RunState,
        utc_now: DateTime<Utc>,
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

#[async_trait]
pub trait SchoolUserCustomUserAgentRepository: Send + Sync {
    async fn add(
        &self,
        owner_user_id: uuid::Uuid,
        student_id: &str,
        user_agent: &str,
    ) -> SchoolRepositoryResult<()>;

    async fn delete(
        &self,
        owner_user_id: uuid::Uuid,
        student_id: &str,
        user_agent: &str,
    ) -> SchoolRepositoryResult<bool>;

    async fn list_by_owner_and_student(
        &self,
        owner_user_id: uuid::Uuid,
        student_id: &str,
    ) -> SchoolRepositoryResult<Vec<String>>;
}
