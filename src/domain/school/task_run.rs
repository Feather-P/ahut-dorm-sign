use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::domain::{
    error::DomainError,
    school::policy::{BusinessDecision, SchoolBusinessDecider},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Pending,
    Waiting,
    Running,
    Finished,
    Failed,
    Expired,
    Canceled,
}

#[derive(Debug, Clone)]
pub struct SchoolSignRun {
    run_id: Uuid,
    task_id: Uuid,
    school_user_id: String,
    date: NaiveDate,
    state: RunState,
    attempt_no: u32,
    scheduled_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    next_retry_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunEvent {
    Enqueue,
    Start,
    Succeed,
    Fail,
    Expire,
    Cancel,
}

impl SchoolSignRun {
    // SchoolSignRun的构造方法，规划一个任务
    pub fn schedule(
        run_id: Uuid,
        task_id: Uuid,
        school_user_id: String,
        date: NaiveDate,
        scheduled_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if school_user_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolUserId);
        }
        Ok(Self {
            run_id,
            task_id,
            school_user_id,
            date,
            state: RunState::Pending,
            attempt_no: 0,
            scheduled_at,
            started_at: None,
            finished_at: None,
            next_retry_at: None,
        })
    }

    /// 处理Event, 进行状态机的状态转移
    pub fn apply(
        &mut self,
        event: RunEvent,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        match event {
            RunEvent::Enqueue => self.enqueue(now),
            RunEvent::Start => self.mark_running(now),
            RunEvent::Succeed => self.mark_succeeded(now),
            // Fail 需要额外的 err + decider，上层请改用 apply_failure_after_error。
            RunEvent::Fail => Err(DomainError::InvalidStateTransition),
            RunEvent::Expire => self.mark_expired(now),
            RunEvent::Cancel => self.cancel(now),
        }
    }

    /// 失败分支专用入口：基于错误语义先决策，再执行状态迁移。
    pub fn apply_failure_after_error(
        &mut self,
        now: DateTime<Utc>,
        err: &DomainError,
        decider: &SchoolBusinessDecider,
        delay_policy: &DelayPolicy,
    ) -> Result<(), DomainError> {
        if !matches!(self.state, RunState::Running) {
            return Err(DomainError::InvalidStateTransition);
        }

        let attempted_retry_times = self.attempt_no.saturating_sub(1);
        match decider.decide_after_error(err, attempted_retry_times) {
            BusinessDecision::Retry => {
                self.state = RunState::Waiting;
                self.next_retry_at = Some(delay_policy.next_retry_at(self.attempt_no, now));
                self.finished_at = None;
                Ok(())
            }
            BusinessDecision::Stop => {
                self.state = RunState::Failed;
                self.finished_at = Some(now);
                self.next_retry_at = None;
                Ok(())
            }
            BusinessDecision::Success => {
                self.state = RunState::Finished;
                self.finished_at = Some(now);
                self.next_retry_at = None;
                Ok(())
            }
        }
    }

    /// 入队准备执行
    fn enqueue(&mut self, _now: DateTime<Utc>) -> Result<(), DomainError> {
        match self.state {
            RunState::Pending => {
                self.state = RunState::Waiting;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition),
        }
    }

    /// 开始执行
    fn mark_running(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if !matches!(self.state, RunState::Waiting) {
            return Err(DomainError::InvalidStateTransition);
        }
        self.state = RunState::Running;
        self.started_at = Some(now);
        self.finished_at = None;
        self.next_retry_at = None;
        self.attempt_no += 1;
        Ok(())
    }

    /// 成功
    fn mark_succeeded(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if !matches!(self.state, RunState::Running) {
            return Err(DomainError::InvalidStateTransition);
        }
        self.state = RunState::Finished;
        self.finished_at = Some(now);
        self.next_retry_at = None;
        Ok(())
    }

    // 过期
    fn mark_expired(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        match self.state {
            RunState::Pending | RunState::Waiting => {
                self.state = RunState::Expired;
                self.finished_at = Some(now);
                self.next_retry_at = None;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition),
        }
    }

    // 取消
    fn cancel(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        match self.state {
            RunState::Pending | RunState::Waiting | RunState::Running => {
                self.state = RunState::Canceled;
                self.finished_at = Some(now);
                self.next_retry_at = None;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition),
        }
    }

    pub fn state(&self) -> RunState {
        self.state
    }
}

pub struct DelayPolicy {
    pub base_delay_seconds: i64,
}

impl DelayPolicy {
    pub fn next_retry_at(&self, attempt_no: u32, now: DateTime<Utc>) -> DateTime<Utc> {
        // 使用指数退避，失败次数越多指数级增长重试间隔
        let safe_attempt = attempt_no.max(1);
        let exp = safe_attempt.saturating_sub(1).min(30);
        let factor = 2_i64.pow(exp);
        let delay = self.base_delay_seconds.saturating_mul(factor);
        now + chrono::Duration::seconds(delay)
    }
}
