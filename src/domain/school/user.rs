use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::school::credential::SchoolCredential;
use crate::domain::school::crypto::{SchoolCredentialProtector, SchoolSidePasswdHasher};

pub struct SchoolUser {
    student_id: String,
    owner_user_id: Uuid,
    user_name: String,
    credential: SchoolCredential,
}

impl SchoolUser {
    pub fn new(
        student_id: String,
        owner_user_id: Uuid,
        user_name: String,
        credential: SchoolCredential,
    ) -> Result<Self, DomainError> {
        if student_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolUserId);
        }
        if user_name.trim().is_empty() {
            return Err(DomainError::BlankUserName);
        }
        Ok(Self {
            student_id,
            owner_user_id,
            user_name,
            credential,
        })
    }

    /// 验证密码
    pub fn verify_password(
        &self,
        plain_password: &str,
        origin_hasher: &dyn SchoolSidePasswdHasher,
        protector: &dyn SchoolCredentialProtector,
    ) -> Result<bool, DomainError> {
        let credential = origin_hasher.hash(plain_password);
        if credential == protector.decrypt(&self.credential)? {
            return Ok(true);
        }
        Ok(false)
    }

    /// 修改密码
    pub fn change_password(
        &mut self,
        old_password: &str,
        new_password: String,
        origin_hasher: &dyn SchoolSidePasswdHasher,
        protector: &dyn SchoolCredentialProtector,
    ) -> Result<(), DomainError> {
        if new_password.trim().is_empty() {
            return Err(DomainError::BlankPassword);
        }
        if self.verify_password(old_password, origin_hasher, protector)? {
            let new_credential = origin_hasher.hash(&new_password);
            self.credential = protector.encrypt(&new_credential);
            Ok(())
        } else {
            Err(DomainError::PasswordMismatch)
        }
    }

    pub fn user_name(&self) -> &str {
        &self.user_name
    }

    pub fn owner_user_id(&self) -> &Uuid {
        &self.owner_user_id
    }

    pub fn student_id(&self) -> &str {
        &self.student_id
    }

    pub fn credential_storage(&self) -> String {
        self.credential.to_storage()
    }

    /// 透传protector的decrypt方法，对用户密码进行解密，通过算法映射回学校原定的md5密码
    pub fn decrypt_credential(
        &self,
        protector: &dyn SchoolCredentialProtector,
    ) -> Result<String, DomainError> {
        protector.decrypt(&self.credential)
    }
}
