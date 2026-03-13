use chrono::{DateTime, Duration, Utc};

use crate::domain::error::DomainError;

pub struct SchoolToken {
    access_token: String,
    refresh_token: String,
    expired_at: DateTime<Utc>,
}

impl SchoolToken {
    pub fn new(
        access_token: String,
        refresh_token: String,
        expired_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if access_token.trim().is_empty() {
            return Err(DomainError::BlankToken);
        }
        if refresh_token.trim().is_empty() {
            return Err(DomainError::BlankToken);
        }
        Ok(Self {
            access_token,
            refresh_token,
            expired_at,
        })
    }

    pub fn refresh(
        &mut self,
        access_token: String,
        refresh_token: String,
        expired_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if access_token.trim().is_empty() {
            return Err(DomainError::BlankToken);
        }
        if refresh_token.trim().is_empty() {
            return Err(DomainError::BlankToken);
        }

        self.access_token = access_token;
        self.refresh_token = refresh_token;
        self.expired_at = expired_at;

        Ok(())
    }

    pub fn is_token_expired(&self, utc_now: DateTime<Utc>) -> bool {
        if utc_now >= self.expired_at {
            return true;
        }
        false
    }

    pub fn need_refresh(&self, utc_now: DateTime<Utc>, refresh_skew: Duration) -> bool {
        if utc_now + refresh_skew >= self.expired_at {
            return true;
        }
        false
    }

    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }

    pub fn expired_at(&self) -> &DateTime<Utc> {
        &self.expired_at
    }
}
