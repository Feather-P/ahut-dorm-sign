use async_trait::async_trait;

use crate::domain::{
    error::DomainError,
    school::{task::{CheckinCommand, SchoolSignTask}, token::SchoolToken, user::SchoolUser},
};

#[async_trait]
pub trait SchoolGateway: Send + Sync {
    async fn authenticate(
        &self,
        user: &SchoolUser,
        hashed_credential: &str,
    ) -> Result<SchoolToken, DomainError>;

    async fn refresh(&self, user: &SchoolUser) -> Result<SchoolToken, DomainError>;

    async fn fetch_active_task_list(&self, user: &SchoolUser) -> Result<Vec<SchoolSignTask>, DomainError>;

    /// 实现本函数的时候应该实现:
    /// 1. 获取微信端点配置
    /// 2. 访问ApiLog留下日志
    async fn prepare_checkin_context(
        &self,
        user: &SchoolUser,
        task_id: &str,
    ) -> Result<(), DomainError>;

    async fn submit_checkin(
        &self,
        user: &SchoolUser,
        target: CheckinCommand,
    ) -> Result<(), DomainError>;
}
