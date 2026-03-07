/// 传输层错误：仅表达 HTTP/序列化/反序列化/底层网络语义。
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("网络库初始化错误: {0}")]
    ClientBuildError(String),

    #[error("网络请求错误: {0}")]
    Request(#[from] reqwest::Error),

    #[error("JSON 序列化失败({field}): {source}")]
    Serialize {
        field: &'static str,
        #[source]
        source: serde_json::Error,
    },

    #[error("JSON 反序列化失败: {0}")]
    Deserialize(#[from] serde_json::Error),

    #[error("HTTP 非成功状态: {status}, body={body}")]
    HttpStatus { status: u16, body: String },
}

/// 服务层错误：表达服务编排/远端业务包裹校验语义。
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("服务[{service}]构建错误: {msg}")]
    BuildError {service: &'static str, msg: String},

    #[error("服务[{service}]请求参数或构建错误: {msg}")]
    InvalidRequest { service: &'static str, msg: String },

    #[error("服务[{service}]远端业务错误: code={code}, msg={msg}")]
    RemoteBusiness {
        service: &'static str,
        code: i32,
        msg: String,
    },
}

/// 领域层错误：表达纯业务领域约束语义。
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("领域校验失败: {0}")]
    Validation(String),
}

/// 对外统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Transport(#[from] TransportError),

    #[error(transparent)]
    Service(#[from] ServiceError),

    #[error(transparent)]
    Domain(#[from] DomainError),
}
