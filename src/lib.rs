mod constants;
mod error;
mod models;
mod network;
mod services;
mod utils;

pub use constants::auth::LOGIN_AUTHORIZATION;
pub use constants::endpoints::BASE_URL;
pub use error::{AppError, DomainError, ServiceError, TransportError};
pub use models::auth::AuthInfo;
pub use models::dorm::DormListData;
pub use network::{AppClient, AppClientBuilder};
pub use services::dorm::list::{DormListRequest, DormListService};
pub use services::login::{LoginRequest, LoginService};
