use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::domain::{error::DomainError, school::token::SchoolToken};

pub struct SchoolSession {
    owner_user_id: Uuid,
    student_id: String,
    token: SchoolToken,
}

impl SchoolSession {
    pub fn new(
        owner_user_id: Uuid,
        student_id: String,
        token: SchoolToken,
    ) -> Result<Self, DomainError> {
        if student_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolUserId);
        }
        Ok(Self {
            owner_user_id,
            student_id,
            token,
        })
    }

    pub fn owner_user_id(&self) -> &Uuid {
        &self.owner_user_id
    }

    pub fn student_id(&self) -> &str {
        &self.student_id
    }

    pub fn access_token(&self) -> &str {
        &self.token.access_token()
    }

    pub fn refresh_token(&self) -> &str {
        &self.token.refresh_token()
    }

    pub fn expired_at(&self) -> DateTime<Utc> {
        *self.token.expired_at()
    }

    pub fn is_expired(&self, utc_now: DateTime<Utc>) -> bool {
        self.token.is_token_expired(utc_now)
    }

    pub fn need_refresh(&self, utc_now: DateTime<Utc>, refresh_skew: Duration) -> bool {
        self.token.need_refresh(utc_now, refresh_skew)
    }

    pub fn replace_token(&mut self, token: SchoolToken) -> () {
        self.token = token
    }
}
