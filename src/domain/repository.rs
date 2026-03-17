use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::access_control::{PermissionCode, RoleCode};
use crate::domain::error::DomainError;
use crate::domain::user::{SystemUser, UserPreferences};

// 系统用户储存
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn list_all_user(&self) -> Result<Vec<SystemUser>, DomainError>;
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<SystemUser>, DomainError>;
    async fn save(&self, user: SystemUser) -> Result<(), DomainError>;
    async fn update_preference(&self, user_id: Uuid, preferences: UserPreferences) -> Result<(), DomainError>;

}

/// 用户-角色绑定
#[async_trait]
pub trait UserRoleBindingRepository: Send + Sync {
    async fn list_roles_of_user(&self, user_id: Uuid) -> Result<Vec<RoleCode>, DomainError>;
    async fn grant_role(&self, user_id: Uuid, role: RoleCode) -> Result<(), DomainError>;
    async fn revoke_role(&self, user_id: Uuid, role: RoleCode) -> Result<(), DomainError>;
    async fn replace_roles(&self, user_id: Uuid, roles: Vec<RoleCode>) -> Result<(), DomainError>;
}

/// 角色-权限映射
#[async_trait]
pub trait RolePermissionRepository: Send + Sync {
    async fn list_permissions_of_role(&self, role: &RoleCode) -> Result<Vec<PermissionCode>, DomainError>;
    async fn grant_permission(&self, role: RoleCode, permission: PermissionCode) -> Result<(), DomainError>;
    async fn revoke_permission(&self, role: RoleCode, permission: PermissionCode) -> Result<(), DomainError>;
    async fn replace_permissions(&self, role: RoleCode, permissions: Vec<PermissionCode>) -> Result<(), DomainError>;
}

/// 角色目录
#[async_trait]
pub trait RoleCatalogRepository: Send + Sync {
    async fn list_all_roles(&self) -> Result<Vec<RoleCode>, DomainError>;
    async fn role_exists(&self, role: &RoleCode) -> Result<bool, DomainError>;
}
