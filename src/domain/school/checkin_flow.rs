use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    error::DomainError,
    school::{
        gateway::SchoolGateway,
        location::GeoPoint,
        policy::{AuthDecision, SchoolAuthDecider},
        repository::{SchoolSessionRepository, SchoolSignTaskRepository, SchoolUserRepository},
        session::SchoolSession,
        task::CheckinCommand,
        user::SchoolUser,
    },
};

pub struct ExecuteCheckinInput {
    pub owner_user_id: Uuid,
    pub student_id: String,
    pub school_task_id: String,
    pub user_agent: String,
    pub point: GeoPoint,
    pub accuracy_meters: f64,
    pub utc_now: DateTime<Utc>,
}

pub struct ExecuteCheckinResult {
    pub school_task_id: String,
    pub task_title: String,
    pub occurred_at_utc: DateTime<Utc>,
}

pub struct SchoolCheckinFlowService<'a> {
    gateway: &'a dyn SchoolGateway,
    user_repo: &'a dyn SchoolUserRepository,
    session_repo: &'a dyn SchoolSessionRepository,
    task_repo: &'a dyn SchoolSignTaskRepository,
    auth_decider: &'a SchoolAuthDecider,
}

impl<'a> SchoolCheckinFlowService<'a> {
    pub fn new(
        gateway: &'a dyn SchoolGateway,
        user_repo: &'a dyn SchoolUserRepository,
        session_repo: &'a dyn SchoolSessionRepository,
        task_repo: &'a dyn SchoolSignTaskRepository,
        auth_decider: &'a SchoolAuthDecider,
    ) -> Self {
        Self {
            gateway,
            user_repo,
            session_repo,
            task_repo,
            auth_decider,
        }
    }

    pub async fn execute(
        &self,
        input: ExecuteCheckinInput,
    ) -> Result<ExecuteCheckinResult, DomainError> {
        if input.student_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolUserId);
        }
        if input.school_task_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolTaskId);
        }
        let user = self
            .load_user(input.owner_user_id, &input.student_id)
            .await?;

        let session = self
            .establish_session(&user, input.owner_user_id, &input.student_id, input.utc_now)
            .await?;

        let selected_ua = input.user_agent;

        let task = self
            .task_repo
            .find_by_student_and_task(&input.student_id, &input.school_task_id)
            .await?
            .ok_or_else(|| DomainError::TaskNotFound {
                task_id: input.school_task_id.clone(),
            })?;

        if !task.is_runnable_at(input.utc_now) {
            return Err(DomainError::NotRunnableNow);
        }

        self.gateway
            .prepare_checkin_context(&session, &input.school_task_id, &selected_ua)
            .await?;

        let command = CheckinCommand::new(
            &input.school_task_id,
            &input.point,
            input.accuracy_meters,
            input.utc_now,
        )?;

        self.gateway
            .submit_checkin(&session, command, &selected_ua)
            .await?;

        Ok(ExecuteCheckinResult {
            school_task_id: input.school_task_id,
            task_title: task.title().to_string(),
            occurred_at_utc: input.utc_now,
        })
    }

    async fn load_user(
        &self,
        owner_user_id: Uuid,
        student_id: &str,
    ) -> Result<SchoolUser, DomainError> {
        self.user_repo
            .find_by_owner_and_student(student_id, owner_user_id)
            .await?
            .ok_or_else(|| DomainError::PersistenceCorrupted {
                message: format!(
                    "缺少校园用户记录：owner_user_id={}, student_id={}",
                    owner_user_id, student_id
                ),
            })
    }

    async fn establish_session(
        &self,
        user: &SchoolUser,
        owner_user_id: Uuid,
        student_id: &str,
        utc_now: DateTime<Utc>,
    ) -> Result<SchoolSession, DomainError> {
        let maybe_session = self
            .session_repo
            .find_by_owner_and_student(owner_user_id, student_id)
            .await?;

        match self.auth_decider.decide_by_session(maybe_session, utc_now) {
            AuthDecision::ReAuthenticate => {
                let token = self.gateway.authenticate(user).await?;
                let session = SchoolSession::new(owner_user_id, student_id.to_owned(), token)?;
                self.session_repo.save(session).await?;
                self.session_repo
                    .find_by_owner_and_student(owner_user_id, student_id)
                    .await?
                    .ok_or_else(|| DomainError::PersistenceCorrupted {
                        message: "会话已持久化但未找到".to_string(),
                    })
            }
            AuthDecision::RefreshToken => {
                let current = self
                    .session_repo
                    .find_by_owner_and_student(owner_user_id, student_id)
                    .await?
                    .ok_or_else(|| DomainError::PersistenceCorrupted {
                        message: "刷新前应存在会话".to_string(),
                    })?;
                let token = self.gateway.refresh(&current).await?;
                let mut refreshed = current;
                refreshed.replace_token(token);
                self.session_repo.save(refreshed).await?;
                self.session_repo
                    .find_by_owner_and_student(owner_user_id, student_id)
                    .await?
                    .ok_or_else(|| DomainError::PersistenceCorrupted {
                        message: "刷新后的会话已持久化但未找到".to_string(),
                    })
            }
            AuthDecision::UseCurrentToken => self
                .session_repo
                .find_by_owner_and_student(owner_user_id, student_id)
                .await?
                .ok_or_else(|| DomainError::PersistenceCorrupted {
                    message: "当前令牌分支应存在会话".to_string(),
                }),
        }
    }

}
