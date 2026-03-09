use uuid::Uuid;

use crate::domain::school::token::SchoolToken;
use crate::domain::{error::DomainError };
use crate::domain::school::crypto::{SchoolPasswordHasher};

pub struct SchoolUser {
    student_id: String,
    owner_user_id: Uuid,
    user_name: String,
    hashed_password: String,
    token: Option<SchoolToken>
}

impl SchoolUser {
    pub fn new(
        student_id: String,
        owner_user_id: Uuid,
        user_name: String,
        hashed_password: String,
    ) -> Result<Self, DomainError> {
        if student_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolUserId);
        }
        if user_name.trim().is_empty() {
            return Err(DomainError::BlankUserName);
        }
        if hashed_password.trim().is_empty() {
            return Err(DomainError::BlankPassword);
        }
        Ok(Self {
            student_id,
            owner_user_id,
            user_name,
            hashed_password,
            token: None
        })
    }

    /// 验证密码
    pub fn verify_password(&self, plain_password: &str, hasher: &dyn SchoolPasswordHasher) -> bool {
        hasher.verify(plain_password, &self.hashed_password)
    }

    /// 修改密码
    pub fn change_password(
        &mut self,
        old_password: &str,
        new_password: String,
        hasher: &dyn SchoolPasswordHasher,
    ) -> Result<(), DomainError> {
        if new_password.trim().is_empty() {
            return Err(DomainError::BlankPassword);
        }
        if self.verify_password(old_password, hasher) {
            self.hashed_password = hasher.hash(&new_password);
            Ok(())
        } else {
            Err(DomainError::PasswordMismatch)
        }
    }

    pub fn update_token(&mut self, token: SchoolToken) {
        match self.token {
            None => {
                self.token = Some(token)
            }
            Some(_) => {
                self.token = Some(token)
            }
        }
    }

    pub fn user_name(&self) -> &str {
        &self.user_name
    }

    pub fn student_id(&self) -> &str {
        &self.student_id
    }
}
