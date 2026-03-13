use async_trait::async_trait;
use chrono::{DateTime, Utc, WeekdaySet};
use sqlx::{PgPool, Row};

use crate::domain::{
    error::DomainError,
    school::{
        credential::SchoolCredential,
        location::GeoPoint,
        repository::{
            SchoolSessionRepository, SchoolSignConfigRepository, SchoolSignTaskRepository,
            SchoolUserRepository,
        },
        session::SchoolSession,
        sign_config::SchoolSignConfig,
        task::{DateRange, SchoolSignTask, TimeWindow},
        token::SchoolToken,
        user::SchoolUser,
    },
};

use super::week_mapper::{parse_school_week, to_school_week};

#[derive(Clone)]
pub struct PgSchoolRepository {
    pool: PgPool,
}

impl PgSchoolRepository {
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

fn map_sqlx_err(err: sqlx::Error) -> DomainError {
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

fn map_row_to_user(row: &sqlx::postgres::PgRow) -> Result<SchoolUser, DomainError> {
    let credential_storage: String = row.get("credential_storage");
    let credential = SchoolCredential::from_storage(&credential_storage)?;

    SchoolUser::new(
        row.get("student_id"),
        row.get("owner_user_id"),
        row.get("user_name"),
        credential,
    )
}

fn map_row_to_sign_config(row: &sqlx::postgres::PgRow) -> Result<SchoolSignConfig, DomainError> {
    let point = GeoPoint::new(row.get("lng"), row.get("lat"))?;
    SchoolSignConfig::new(
        row.get("id"),
        row.get("owner_user_id"),
        row.get("student_id"),
        row.get("school_task_id"),
        point,
        row.get("jitter_radius_min_meters"),
        row.get("jitter_radius_max_meters"),
        row.get("accuracy_min_meters"),
        row.get("accuracy_max_meters"),
        row.get("allow_sign_timerange_start"),
        row.get("allow_sign_timerange_end"),
        row.get("enable"),
    )
}

fn map_row_to_sign_task(row: &sqlx::postgres::PgRow) -> Result<SchoolSignTask, DomainError> {
    let days_of_week = parse_school_week(&row.get::<String, _>("days_of_week"));
    let date_range = DateRange::new(row.get("date_start"), row.get("date_end"))?;
    let time_window = TimeWindow::new(row.get("time_start"), row.get("time_end"))?;
    let time_zone = row
        .get::<String, _>("time_zone")
        .parse::<chrono_tz::Tz>()
        .unwrap_or(chrono_tz::Asia::Shanghai);

    SchoolSignTask::new(
        row.get("id"),
        row.get("student_id"),
        row.get("school_task_id"),
        row.get("title"),
        date_range,
        time_window,
        days_of_week,
        time_zone,
    )
}

fn map_row_to_session(row: &sqlx::postgres::PgRow) -> Result<SchoolSession, DomainError> {
    let token = SchoolToken::new(
        row.get("access_token"),
        row.get("refresh_token"),
        row.get("expired_at"),
    )?;
    SchoolSession::new(row.get("owner_user_id"), row.get("student_id"), token)
}

fn weekdays_to_storage(weekdays: &WeekdaySet) -> String {
    let mut days = Vec::new();
    for w in [
        chrono::Weekday::Mon,
        chrono::Weekday::Tue,
        chrono::Weekday::Wed,
        chrono::Weekday::Thu,
        chrono::Weekday::Fri,
        chrono::Weekday::Sat,
        chrono::Weekday::Sun,
    ] {
        if weekdays.contains(w) {
            days.push(to_school_week(w));
        }
    }
    days.join(",")
}

#[async_trait]
impl SchoolUserRepository for PgSchoolRepository {
    async fn find_by_owner_and_student(
        &self,
        student_id: &str,
        owner_user_id: uuid::Uuid,
    ) -> Result<Option<SchoolUser>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT student_id, owner_user_id, user_name, credential_storage
            FROM school_users
            WHERE student_id = $1 AND owner_user_id = $2
            "#,
        )
        .bind(student_id)
        .bind(owner_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        row.map(|r| map_row_to_user(&r)).transpose()
    }

    async fn list_by_owner_user_id(
        &self,
        owner_user_id: uuid::Uuid,
    ) -> Result<Vec<SchoolUser>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT student_id, owner_user_id, user_name, credential_storage
            FROM school_users
            WHERE owner_user_id = $1
            ORDER BY student_id
            "#,
        )
        .bind(owner_user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        rows.iter().map(map_row_to_user).collect()
    }

    async fn save(&self, user: SchoolUser) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO school_users (student_id, owner_user_id, user_name, credential_storage, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (student_id, owner_user_id)
            DO UPDATE SET
              user_name = EXCLUDED.user_name,
              credential_storage = EXCLUDED.credential_storage,
              updated_at = NOW()
            "#,
        )
        .bind(user.student_id())
        .bind(user.owner_user_id())
        .bind(user.user_name())
        .bind(user.credential_storage())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn delete_by_student_id(&self, student_id: &str) -> Result<bool, DomainError> {
        let affected = sqlx::query("DELETE FROM school_users WHERE student_id = $1")
            .bind(student_id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        Ok(affected > 0)
    }
}

#[async_trait]
impl SchoolSignConfigRepository for PgSchoolRepository {
    async fn find_enabled_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> Result<Option<SchoolSignConfig>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT * FROM school_sign_configs
            WHERE student_id = $1 AND school_task_id = $2 AND enable = TRUE
            "#,
        )
        .bind(student_id)
        .bind(school_task_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        row.map(|r| map_row_to_sign_config(&r)).transpose()
    }

    async fn find_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> Result<Option<SchoolSignConfig>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT * FROM school_sign_configs
            WHERE student_id = $1 AND school_task_id = $2
            "#,
        )
        .bind(student_id)
        .bind(school_task_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        row.map(|r| map_row_to_sign_config(&r)).transpose()
    }

    async fn list_by_student_id(
        &self,
        student_id: &str,
    ) -> Result<Vec<SchoolSignConfig>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM school_sign_configs WHERE student_id = $1 ORDER BY school_task_id",
        )
        .bind(student_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        rows.iter().map(map_row_to_sign_config).collect()
    }

    async fn list_enabled_by_student_id(
        &self,
        student_id: &str,
    ) -> Result<Vec<SchoolSignConfig>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM school_sign_configs WHERE student_id = $1 AND enable = TRUE ORDER BY school_task_id",
        )
        .bind(student_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        rows.iter().map(map_row_to_sign_config).collect()
    }

    async fn save(&self, config: SchoolSignConfig) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO school_sign_configs (
              id, owner_user_id, student_id, school_task_id,
              lng, lat,
              jitter_radius_min_meters, jitter_radius_max_meters,
              accuracy_min_meters, accuracy_max_meters,
              allow_sign_timerange_start, allow_sign_timerange_end,
              enable, version, created_at, updated_at
            ) VALUES (
              $1, $2, $3, $4,
              $5, $6,
              $7, $8,
              $9, $10,
              $11, $12,
              $13, 0, NOW(), NOW()
            )
            ON CONFLICT (student_id, school_task_id)
            DO UPDATE SET
              owner_user_id = EXCLUDED.owner_user_id,
              lng = EXCLUDED.lng,
              lat = EXCLUDED.lat,
              jitter_radius_min_meters = EXCLUDED.jitter_radius_min_meters,
              jitter_radius_max_meters = EXCLUDED.jitter_radius_max_meters,
              accuracy_min_meters = EXCLUDED.accuracy_min_meters,
              accuracy_max_meters = EXCLUDED.accuracy_max_meters,
              allow_sign_timerange_start = EXCLUDED.allow_sign_timerange_start,
              allow_sign_timerange_end = EXCLUDED.allow_sign_timerange_end,
              enable = EXCLUDED.enable,
              version = school_sign_configs.version + 1,
              updated_at = NOW()
            "#,
        )
        .bind(config.id)
        .bind(config.owner_user_id)
        .bind(config.student_id)
        .bind(config.school_task_id)
        .bind(config.point.lng())
        .bind(config.point.lat())
        .bind(config.jitter_radius_min_meters)
        .bind(config.jitter_radius_max_meters)
        .bind(config.accuracy_min_meters)
        .bind(config.accuracy_max_meters)
        .bind(config.allow_sign_timerange_start)
        .bind(config.allow_sign_timerange_end)
        .bind(config.enable)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn delete_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> Result<bool, DomainError> {
        let affected = sqlx::query(
            "DELETE FROM school_sign_configs WHERE student_id = $1 AND school_task_id = $2",
        )
        .bind(student_id)
        .bind(school_task_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    async fn set_enabled(
        &self,
        student_id: &str,
        school_task_id: &str,
        enabled: bool,
    ) -> Result<bool, DomainError> {
        let affected = sqlx::query(
            r#"
            UPDATE school_sign_configs
            SET enable = $3, version = version + 1, updated_at = NOW()
            WHERE student_id = $1 AND school_task_id = $2
            "#,
        )
        .bind(student_id)
        .bind(school_task_id)
        .bind(enabled)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        Ok(affected > 0)
    }
}

#[async_trait]
impl SchoolSignTaskRepository for PgSchoolRepository {
    async fn find_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> Result<Option<SchoolSignTask>, DomainError> {
        let row = sqlx::query(
            "SELECT * FROM school_sign_tasks WHERE student_id = $1 AND school_task_id = $2",
        )
        .bind(student_id)
        .bind(school_task_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        row.map(|r| map_row_to_sign_task(&r)).transpose()
    }

    async fn find_runnable(
        &self,
        student_id: &str,
        utc_now: DateTime<Utc>,
    ) -> Result<Option<SchoolSignTask>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM school_sign_tasks WHERE student_id = $1 ORDER BY school_task_id",
        )
        .bind(student_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        for row in rows {
            let task = map_row_to_sign_task(&row)?;
            if task.is_runnable_at(utc_now) {
                return Ok(Some(task));
            }
        }
        Ok(None)
    }

    async fn list_by_student_id(
        &self,
        student_id: &str,
    ) -> Result<Vec<SchoolSignTask>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM school_sign_tasks WHERE student_id = $1 ORDER BY school_task_id",
        )
        .bind(student_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        rows.iter().map(map_row_to_sign_task).collect()
    }

    async fn save(&self, sign_task: SchoolSignTask) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO school_sign_tasks (
              id, student_id, school_task_id, title,
              date_start, date_end, time_start, time_end,
              days_of_week, time_zone,
              created_at, updated_at
            ) VALUES (
              $1, $2, $3, $4,
              $5, $6, $7, $8,
              $9, $10,
              NOW(), NOW()
            )
            ON CONFLICT (student_id, school_task_id)
            DO UPDATE SET
              title = EXCLUDED.title,
              date_start = EXCLUDED.date_start,
              date_end = EXCLUDED.date_end,
              time_start = EXCLUDED.time_start,
              time_end = EXCLUDED.time_end,
              days_of_week = EXCLUDED.days_of_week,
              time_zone = EXCLUDED.time_zone,
              updated_at = NOW()
            "#,
        )
        .bind(sign_task.id())
        .bind(sign_task.student_id())
        .bind(sign_task.school_task_id())
        .bind(sign_task.title())
        .bind(sign_task.date_range().start())
        .bind(sign_task.date_range().end())
        .bind(sign_task.daily_time_window().start())
        .bind(sign_task.daily_time_window().end())
        .bind(weekdays_to_storage(sign_task.days_of_week()))
        .bind(sign_task.time_zone().name())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn delete_by_student_and_task(
        &self,
        student_id: &str,
        school_task_id: &str,
    ) -> Result<bool, DomainError> {
        let affected = sqlx::query(
            "DELETE FROM school_sign_tasks WHERE student_id = $1 AND school_task_id = $2",
        )
        .bind(student_id)
        .bind(school_task_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        Ok(affected > 0)
    }
}

#[async_trait]
impl SchoolSessionRepository for PgSchoolRepository {
    async fn find_by_owner_and_student(
        &self,
        owner_user_id: uuid::Uuid,
        student_id: &str,
    ) -> Result<Option<SchoolSession>, DomainError> {
        let row = sqlx::query(
            "SELECT owner_user_id, student_id, access_token, refresh_token, expired_at FROM school_sessions WHERE owner_user_id = $1 AND student_id = $2",
        )
        .bind(owner_user_id)
        .bind(student_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        row.map(|r| map_row_to_session(&r)).transpose()
    }

    async fn save(&self, session: SchoolSession) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO school_sessions (
              owner_user_id, student_id, access_token, refresh_token, expired_at, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
            ON CONFLICT (owner_user_id, student_id)
            DO UPDATE SET
              access_token = EXCLUDED.access_token,
              refresh_token = EXCLUDED.refresh_token,
              expired_at = EXCLUDED.expired_at,
              updated_at = NOW()
            "#,
        )
        .bind(session.owner_user_id())
        .bind(session.student_id())
        .bind(session.access_token())
        .bind(session.refresh_token())
        .bind(session.expired_at())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn delete_by_owner_and_student(
        &self,
        owner_user_id: uuid::Uuid,
        student_id: &str,
    ) -> Result<bool, DomainError> {
        let affected =
            sqlx::query("DELETE FROM school_sessions WHERE owner_user_id = $1 AND student_id = $2")
                .bind(owner_user_id)
                .bind(student_id)
                .execute(&self.pool)
                .await
                .map_err(map_sqlx_err)?
                .rows_affected();
        Ok(affected > 0)
    }
}
