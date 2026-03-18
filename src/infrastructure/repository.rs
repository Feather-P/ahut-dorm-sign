use async_trait::async_trait;
use chrono_tz::Tz;
use sqlx::{PgPool, Row};

use crate::domain::{
    access_control::{PermissionCode, RoleCode},
    error::DomainError,
    repository::{
        RoleCatalogRepository, RolePermissionRepository, UserRepository, UserRoleBindingRepository,
    },
    user::{SystemUser, UserPreferences},
};

// 给PgPool包一层抽象，虽然感觉好像没什么用，但是大概更方便罢（
#[derive(Clone)]
pub struct PgRepositoryBase {
    pool: PgPool,
}

impl PgRepositoryBase {
    pub async fn connect(database_url: &str) -> Result<Self, DomainError> {
        let pool = PgPool::connect(database_url).await.map_err(map_sqlx_err)?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

pub fn map_sqlx_err(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::PoolTimedOut
        | sqlx::Error::PoolClosed
        | sqlx::Error::Io(_)
        | sqlx::Error::Tls(_)
        | sqlx::Error::WorkerCrashed => DomainError::PersistenceUnavailable {
            message: format!("PostgreSQL 不可用: {err}"),
        },
        sqlx::Error::RowNotFound => DomainError::PersistenceCorrupted {
            message: "在非预期上下文中未找到记录".to_string(),
        },
        sqlx::Error::Database(db_err) => {
            if let Some(code) = db_err.code() {
                match code.as_ref() {
                    // serialization_failure / deadlock_detected / lock_not_available
                    "40001" | "40P01" | "55P03" => DomainError::PersistenceConflict {
                        message: format!("PostgreSQL 并发冲突({code}): {}", db_err.message()),
                    },
                    // unique_violation / exclusion_violation
                    "23505" | "23P01" => DomainError::PersistenceConflict {
                        message: format!("PostgreSQL 约束冲突({code}): {}", db_err.message()),
                    },
                    _ => DomainError::PersistenceCorrupted {
                        message: format!("PostgreSQL 数据库错误({code}): {}", db_err.message()),
                    },
                }
            } else {
                DomainError::PersistenceCorrupted {
                    message: format!("PostgreSQL 数据库错误: {}", db_err.message()),
                }
            }
        }
        other => DomainError::PersistenceCorrupted {
            message: format!("PostgreSQL 未预期错误: {other}"),
        },
    }
}

pub struct PgSystemRepository {
    base: PgRepositoryBase,
}

impl PgSystemRepository {
    pub async fn connect(database_url: &str) -> Result<Self, DomainError> {
        let pool = PgRepositoryBase::connect(database_url).await?;
        Ok(Self { base: pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self {
            base: PgRepositoryBase::from_pool(pool),
        }
    }

    pub fn pool(&self) -> &PgPool {
        self.base.pool()
    }
}

fn map_row_to_user(row: &sqlx::postgres::PgRow) -> Result<SystemUser, DomainError> {
    let tz_str: String = row.get("time_zone");
    let tz: Tz = tz_str
        .parse()
        .map_err(|e| DomainError::PersistenceCorrupted {
            message: format!("无效时区数据 time_zone={tz_str}, err={e}"),
        })?;

    let preferences = UserPreferences::new(tz);

    SystemUser::new(row.get("id"), row.get("user_name"), preferences)
}

fn map_row_to_role_code(row: &sqlx::postgres::PgRow) -> Result<RoleCode, DomainError> {
    let role_code: String = row.get("code");
    RoleCode::new(&role_code).map_err(|e| DomainError::PersistenceCorrupted {
        message: format!("无效角色码 code={role_code}, err={e}"),
    })
}

fn map_row_to_bound_role_code(row: &sqlx::postgres::PgRow) -> Result<RoleCode, DomainError> {
    let role_code: String = row.get("role_code");
    RoleCode::new(&role_code).map_err(|e| DomainError::PersistenceCorrupted {
        message: format!("无效角色码 role_code={role_code}, err={e}"),
    })
}

fn map_row_to_permission_code(row: &sqlx::postgres::PgRow) -> Result<PermissionCode, DomainError> {
    let permission_code: String = row.get("permission_code");
    PermissionCode::new(&permission_code).map_err(|e| DomainError::PersistenceCorrupted {
        message: format!("无效权限码 permission_code={permission_code}, err={e}"),
    })
}

#[async_trait]
impl UserRepository for PgSystemRepository {
    async fn list_all_user(&self) -> Result<Vec<SystemUser>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_name, time_zone
            FROM users
            ORDER BY user_name, id
            "#,
        )
        .fetch_all(self.pool())
        .await
        .map_err(map_sqlx_err)?;

        rows.iter().map(map_row_to_user).collect()
    }

    async fn find_by_id(&self, user_id: uuid::Uuid) -> Result<Option<SystemUser>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT id, user_name, time_zone
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_err)?;

        row.map(|r| map_row_to_user(&r)).transpose()
    }

    async fn save(&self, user: SystemUser) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO users (id, user_name, time_zone, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            ON CONFLICT (id)
            DO UPDATE SET
              user_name = EXCLUDED.user_name,
              time_zone = EXCLUDED.time_zone,
              updated_at = NOW()
            "#,
        )
        .bind(user.id())
        .bind(user.username())
        .bind(user.time_zone().name())
        .execute(self.pool())
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn update_preference(
        &self,
        user_id: uuid::Uuid,
        preferences: UserPreferences,
    ) -> Result<(), DomainError> {
        let affected = sqlx::query(
            r#"
            UPDATE users
            SET time_zone = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(preferences.time_zone().name())
        .execute(self.pool())
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if affected == 0 {
            return Err(DomainError::PersistenceCorrupted {
                message: format!("更新用户偏好失败，用户不存在: user_id={user_id}"),
            });
        }

        Ok(())
    }
}

#[async_trait]
impl RoleCatalogRepository for PgSystemRepository {
    async fn list_all_roles(&self) -> Result<Vec<RoleCode>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT code
            FROM roles
            ORDER BY code
            "#,
        )
        .fetch_all(self.pool())
        .await
        .map_err(map_sqlx_err)?;

        rows.iter().map(map_row_to_role_code).collect()
    }

    async fn role_exists(&self, role: &RoleCode) -> Result<bool, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM roles WHERE code = $1
            ) AS exist
            "#,
        )
        .bind(role.as_str())
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.get::<bool, _>("exist"))
    }
}

#[async_trait]
impl UserRoleBindingRepository for PgSystemRepository {
    async fn list_roles_of_user(&self, user_id: uuid::Uuid) -> Result<Vec<RoleCode>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT role_code
            FROM user_roles
            WHERE user_id = $1
            ORDER BY role_code
            "#,
        )
        .bind(user_id)
        .fetch_all(self.pool())
        .await
        .map_err(map_sqlx_err)?;

        rows.iter().map(map_row_to_bound_role_code).collect()
    }

    async fn grant_role(&self, user_id: uuid::Uuid, role: RoleCode) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO user_roles (user_id, role_code)
            VALUES ($1, $2)
            ON CONFLICT (user_id, role_code) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(role.as_str())
        .execute(self.pool())
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn revoke_role(&self, user_id: uuid::Uuid, role: RoleCode) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            DELETE FROM user_roles
            WHERE user_id = $1 AND role_code = $2
            "#,
        )
        .bind(user_id)
        .bind(role.as_str())
        .execute(self.pool())
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn replace_roles(
        &self,
        user_id: uuid::Uuid,
        roles: Vec<RoleCode>,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool().begin().await.map_err(map_sqlx_err)?;

        sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
            .bind(user_id)
            .execute(tx.as_mut())
            .await
            .map_err(map_sqlx_err)?;

        for role in roles {
            sqlx::query(
                r#"
                INSERT INTO user_roles (user_id, role_code)
                VALUES ($1, $2)
                ON CONFLICT (user_id, role_code) DO NOTHING
                "#,
            )
            .bind(user_id)
            .bind(role.as_str())
            .execute(tx.as_mut())
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;

        Ok(())
    }
}

#[async_trait]
impl RolePermissionRepository for PgSystemRepository {
    async fn list_permissions_of_role(
        &self,
        role: &RoleCode,
    ) -> Result<Vec<PermissionCode>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT permission_code
            FROM role_permissions
            WHERE role_code = $1
            ORDER BY permission_code
            "#,
        )
        .bind(role.as_str())
        .fetch_all(self.pool())
        .await
        .map_err(map_sqlx_err)?;

        rows.iter().map(map_row_to_permission_code).collect()
    }

    async fn grant_permission(
        &self,
        role: RoleCode,
        permission: PermissionCode,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO role_permissions (role_code, permission_code)
            VALUES ($1, $2)
            ON CONFLICT (role_code, permission_code) DO NOTHING
            "#,
        )
        .bind(role.as_str())
        .bind(permission.as_str())
        .execute(self.pool())
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn revoke_permission(
        &self,
        role: RoleCode,
        permission: PermissionCode,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            DELETE FROM role_permissions
            WHERE role_code = $1 AND permission_code = $2
            "#,
        )
        .bind(role.as_str())
        .bind(permission.as_str())
        .execute(self.pool())
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn replace_permissions(
        &self,
        role: RoleCode,
        permissions: Vec<PermissionCode>,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool().begin().await.map_err(map_sqlx_err)?;

        sqlx::query("DELETE FROM role_permissions WHERE role_code = $1")
            .bind(role.as_str())
            .execute(tx.as_mut())
            .await
            .map_err(map_sqlx_err)?;

        for permission in permissions {
            sqlx::query(
                r#"
                INSERT INTO role_permissions (role_code, permission_code)
                VALUES ($1, $2)
                ON CONFLICT (role_code, permission_code) DO NOTHING
                "#,
            )
            .bind(role.as_str())
            .bind(permission.as_str())
            .execute(tx.as_mut())
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;

        Ok(())
    }
}
