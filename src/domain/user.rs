use uuid::Uuid;

use crate::domain::error::DomainError;

#[derive(Debug, Clone)]
pub struct SystemUser {
    id: Uuid,
    username: String,
    time_zone: chrono_tz::Tz,
}

impl SystemUser {
    pub fn new(
        id: Uuid,
        username: String,
        time_zone: chrono_tz::Tz,
    ) -> Result<Self, DomainError> {
        if username.trim().is_empty() {
            return Err(DomainError::BlankUserName);
        }
        Ok(Self {
            id,
            username,
            time_zone,
        })
    }

    pub fn id(&self) -> Uuid { self.id }
    pub fn username(&self) -> &str { &self.username }
    pub fn time_zone(&self) -> chrono_tz::Tz { self.time_zone }
}
