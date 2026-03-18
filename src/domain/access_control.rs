use std::collections::{BTreeMap, BTreeSet};

use crate::domain::error::DomainError;
use crate::domain::user::SystemUser;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PermissionCode(String);

impl PermissionCode {
    pub fn new(code: &str) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::BlankPermissionCode);
        }
        Ok(Self(code.trim().to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn user_read_own() -> Self {
        Self("user.read.own".to_string())
    }

    pub fn user_manage_own() -> Self {
        Self("user.manage.own".to_string())
    }

    pub fn user_manage_global() -> Self {
        Self("user.manage.global".to_string())
    }

    pub fn task_read_own() -> Self {
        Self("task.read.own".to_string())
    }

    pub fn task_manage_own() -> Self {
        Self("task.manage.own".to_string())
    }

    pub fn task_manage_global() -> Self {
        Self("task.manage.global".to_string())
    }

    pub fn session_read_own() -> Self {
        Self("session.read.own".to_string())
    }

    pub fn session_manage_own() -> Self {
        Self("session.manage.own".to_string())
    }

    pub fn session_manage_global() -> Self {
        Self("session.manage.global".to_string())
    }

    pub fn admin_read_global() -> Self {
        Self("admin.read.global".to_string())
    }

    pub fn parts(&self) -> Option<PermissionParts<'_>> {
        let mut parts = self.as_str().split('.');
        let resource = parts.next()?;
        let action = parts.next()?;
        let scope = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        Some(PermissionParts {
            resource,
            action,
            scope,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PermissionParts<'a> {
    pub resource: &'a str,
    pub action: &'a str,
    pub scope: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoleCode(String);

impl RoleCode {
    pub fn new(code: &str) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::BlankRoleCode);
        }
        Ok(Self(code.trim().to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn super_admin() -> Self {
        Self("super_admin".to_string())
    }

    pub fn admin() -> Self {
        Self("admin".to_string())
    }

    pub fn user() -> Self {
        Self("user".to_string())
    }
}

#[derive(Debug, Clone)]
pub struct UserRoleBindings {
    inner: BTreeMap<uuid::Uuid, BTreeSet<RoleCode>>,
}

impl UserRoleBindings {
    pub fn empty() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    pub fn roles_of(&self, user: &SystemUser) -> BTreeSet<RoleCode> {
        self.inner.get(&user.id()).cloned().unwrap_or_default()
    }

    pub fn grant(&mut self, user: &SystemUser, role: RoleCode) {
        self.inner.entry(user.id()).or_default().insert(role);
    }

    pub fn revoke(&mut self, user: &SystemUser, role: &RoleCode) {
        if let Some(roles) = self.inner.get_mut(&user.id()) {
            roles.remove(role);
        }
    }
}

#[derive(Debug, Clone)]
pub struct RolePermissions {
    inner: BTreeMap<RoleCode, BTreeSet<PermissionCode>>,
}

impl RolePermissions {
    pub fn default_mvp() -> Self {
        let mut inner = BTreeMap::new();

        inner.insert(
            RoleCode::super_admin(),
            BTreeSet::from([
                PermissionCode::user_manage_global(),
                PermissionCode::task_manage_global(),
                PermissionCode::session_manage_global(),
                PermissionCode::admin_read_global(),
            ]),
        );

        inner.insert(
            RoleCode::admin(),
            BTreeSet::from([
                PermissionCode::user_manage_global(),
                PermissionCode::task_manage_global(),
                PermissionCode::session_manage_global(),
            ]),
        );

        inner.insert(
            RoleCode::user(),
            BTreeSet::from([
                PermissionCode::task_read_own(),
                PermissionCode::session_read_own(),
                PermissionCode::user_manage_own(),
                PermissionCode::task_manage_own(),
                PermissionCode::session_manage_own(),
            ]),
        );

        Self { inner }
    }

    pub fn permissions_of(&self, role: &RoleCode) -> BTreeSet<PermissionCode> {
        self.inner.get(role).cloned().unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub enum ResourceRef {
    Task { owner_user_id: Option<uuid::Uuid> },
    Session { owner_user_id: Option<uuid::Uuid> },
    SchoolUser { owner_user_id: Option<uuid::Uuid> },
    User { target_user_id: uuid::Uuid },
    Admin,
}

#[derive(Debug, Clone)]
pub struct AccessRequest {
    need: PermissionCode,
    resource: ResourceRef,
}

impl AccessRequest {
    pub fn new(need: PermissionCode, resource: ResourceRef) -> Self {
        Self { need, resource }
    }

    pub fn need(&self) -> &PermissionCode {
        &self.need
    }

    pub fn resource(&self) -> &ResourceRef {
        &self.resource
    }
}

pub struct AccessPolicy;

impl AccessPolicy {
    pub fn can(
        user: &SystemUser,
        req: &AccessRequest,
        role_permissions: &RolePermissions,
        user_role_bindings: &UserRoleBindings,
    ) -> bool {
        let needed = match req.need().parts() {
            Some(parts) => parts,
            None => return false,
        };

        user_role_bindings
            .roles_of(user)
            .into_iter()
            .flat_map(|r| role_permissions.permissions_of(&r))
            .any(|p| {
                p.parts().is_some_and(|grant| {
                    Self::permission_implies(grant, needed)
                        && Self::scope_constraint_allows(user, req)
                })
            })
    }

    pub fn enforce(
        user: &SystemUser,
        req: &AccessRequest,
        role_permissions: &RolePermissions,
        user_role_bindings: &UserRoleBindings,
    ) -> Result<(), DomainError> {
        if Self::can(user, req, role_permissions, user_role_bindings) {
            Ok(())
        } else {
            Err(DomainError::PermissionDenied)
        }
    }

    fn permission_implies(grant: PermissionParts<'_>, need: PermissionParts<'_>) -> bool {
        let resource_match = grant.resource == need.resource || grant.resource == "*";
        let action_match =
            grant.action == need.action || grant.action == "manage" || grant.action == "*";
        let scope_match = match (grant.scope, need.scope) {
            ("*", _) => true,
            ("global", "global") => true,
            ("global", "own") => true,
            (g, n) => g == n,
        };
        resource_match && action_match && scope_match
    }

    fn scope_constraint_allows(user: &SystemUser, req: &AccessRequest) -> bool {
        let needed = match req.need().parts() {
            Some(parts) => parts,
            None => return false,
        };

        match needed.scope {
            "global" | "*" => true,
            "own" => match req.resource() {
                ResourceRef::Task { owner_user_id } => {
                    owner_user_id.is_some_and(|id| id == user.id())
                }
                ResourceRef::Session { owner_user_id } => {
                    owner_user_id.is_some_and(|id| id == user.id())
                }
                ResourceRef::SchoolUser { owner_user_id } => {
                    owner_user_id.is_some_and(|id| id == user.id())
                }
                ResourceRef::User { target_user_id } => *target_user_id == user.id(),
                ResourceRef::Admin => false,
            },
            _ => false,
        }
    }
}
