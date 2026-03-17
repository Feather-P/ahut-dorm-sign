use uuid::Uuid;

use crate::domain::error::DomainError;

#[derive(Debug, Clone)]
pub struct UserPreferences {
    time_zone: chrono_tz::Tz,
}

impl UserPreferences {
    pub fn new(time_zone: chrono_tz::Tz) -> Self { Self { time_zone } }

    pub fn time_zone(&self) -> chrono_tz::Tz { self.time_zone }
}

#[derive(Debug, Clone)]
pub struct SystemUser {
    id: Uuid,
    username: String,
    preferences: UserPreferences,
}

impl SystemUser {
    pub fn new(
        id: Uuid,
        username: String,
        preferences: UserPreferences,
    ) -> Result<Self, DomainError> {
        if username.trim().is_empty() {
            return Err(DomainError::BlankUserName);
        }
        Ok(Self {
            id,
            username,
            preferences,
        })
    }

    pub fn id(&self) -> Uuid { self.id }
    pub fn username(&self) -> &str { &self.username }
    pub fn preferences(&self) -> &UserPreferences { &self.preferences }
    pub fn time_zone(&self) -> chrono_tz::Tz { self.preferences.time_zone() }
}
