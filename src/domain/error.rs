use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSource {
    Local,
    School,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Retryable,
    ReauthRequired,
    Terminal,
    IdempotentSuccess,
}

#[derive(Debug, Error, PartialEq)]
pub enum DomainError {
    #[error("任务现在不可运行")]
    NotRunnableNow,

    #[error("无效的密码")]
    InvalidPassword,

    #[error("日期范围无效")]
    InvalidDateRange,

    #[error("时间窗口无效")]
    InvalidTimeWindow,

    #[error("学校用户ID不能为空")]
    BlankSchoolUserId,

    #[error("学校任务ID不能为空")]
    BlankSchoolTaskId,

    #[error("用户名不能为空")]
    BlankUserName,

    #[error("密码不能为空")]
    BlankPassword,

    #[error("标题不能为空")]
    BlankTitle,

    #[error("星期不能为空")]
    BlankDaysOfWeek,

    #[error("无效的位置: lng {0}, lat {1}")]
    InvalidCoordinates(f64, f64),

    #[error("无效的定位精度: {0}")]
    InvalidLocationAccuracy(f64),

    #[error("无效的token刷新提前量(秒): {0}")]
    InvalidTokenRefreshSkewSeconds(i64),

    #[error("无效的重试次数: {0}")]
    InvalidRetryTimes(u32),

    #[error("签到配置已禁用")]
    SignConfigDisabled,

    #[error("token不能为空")]
    BlankToken,

    #[error("密码不正确")]
    PasswordMismatch,

    #[error("未找到签到任务: {task_id}")]
    TaskNotFound { task_id: String },

    #[error("认证失败")]
    Unauthorized { origin: ErrorSource },

    #[error("会话已过期")]
    TokenExpired { origin: ErrorSource },

    #[error("远端请求超时")]
    RemoteTimeout { origin: ErrorSource },

    #[error("远端服务不可用")]
    RemoteUnavailable { origin: ErrorSource },

    #[error("已完成签到")]
    AlreadySigned { origin: ErrorSource },

    #[error("上游系统拒绝请求: code={code:?}, message={message}")]
    UpstreamRejected {
        origin: ErrorSource,
        code: Option<i64>,
        message: String,
    },
}

impl DomainError {
    pub fn kind(&self) -> ErrorKind {
        match self {
            DomainError::NotRunnableNow
            | DomainError::InvalidPassword
            | DomainError::InvalidDateRange
            | DomainError::InvalidTimeWindow
            | DomainError::BlankSchoolUserId
            | DomainError::BlankSchoolTaskId
            | DomainError::BlankUserName
            | DomainError::BlankPassword
            | DomainError::BlankTitle
            | DomainError::BlankDaysOfWeek
            | DomainError::InvalidCoordinates(_, _)
            | DomainError::InvalidLocationAccuracy(_)
            | DomainError::InvalidTokenRefreshSkewSeconds(_)
            | DomainError::InvalidRetryTimes(_)
            | DomainError::SignConfigDisabled
            | DomainError::BlankToken
            | DomainError::PasswordMismatch
            | DomainError::TaskNotFound { .. } => ErrorKind::Terminal,
            DomainError::Unauthorized { .. } | DomainError::TokenExpired { .. } => {
                ErrorKind::ReauthRequired
            }
            DomainError::RemoteTimeout { .. } | DomainError::RemoteUnavailable { .. } => {
                ErrorKind::Retryable
            }
            DomainError::AlreadySigned { .. } => ErrorKind::IdempotentSuccess,
            DomainError::UpstreamRejected { code, .. } => match code {
                Some(401) | Some(403) => ErrorKind::ReauthRequired,
                Some(408) | Some(429) | Some(500..=599) => ErrorKind::Retryable,
                _ => ErrorKind::Terminal,
            },
        }
    }

    pub fn source(&self) -> ErrorSource {
        match self {
            DomainError::NotRunnableNow
            | DomainError::InvalidPassword
            | DomainError::InvalidDateRange
            | DomainError::InvalidTimeWindow
            | DomainError::BlankSchoolUserId
            | DomainError::BlankSchoolTaskId
            | DomainError::BlankUserName
            | DomainError::BlankPassword
            | DomainError::BlankTitle
            | DomainError::BlankDaysOfWeek
            | DomainError::InvalidCoordinates(_, _)
            | DomainError::InvalidLocationAccuracy(_)
            | DomainError::InvalidTokenRefreshSkewSeconds(_)
            | DomainError::InvalidRetryTimes(_)
            | DomainError::SignConfigDisabled
            | DomainError::BlankToken
            | DomainError::PasswordMismatch
            | DomainError::TaskNotFound { .. } => ErrorSource::Local,
            DomainError::Unauthorized { origin }
            | DomainError::TokenExpired { origin }
            | DomainError::RemoteTimeout { origin }
            | DomainError::RemoteUnavailable { origin }
            | DomainError::AlreadySigned { origin }
            | DomainError::UpstreamRejected { origin, .. } => *origin,
        }
    }
}
