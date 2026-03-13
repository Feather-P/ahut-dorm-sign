use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    error::DomainError,
    school::{location::GeoPoint, noise::CheckinNoiseGenerator, task::CheckinCommand},
};

pub struct SchoolSignConfig {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub student_id: String,
    pub school_task_id: String,
    pub point: GeoPoint,
    pub jitter_radius_min_meters: f64,
    pub jitter_radius_max_meters: f64,
    pub accuracy_min_meters: f64,
    pub accuracy_max_meters: f64,
    pub allow_sign_timerange_start: DateTime<Utc>,
    pub allow_sign_timerange_end: DateTime<Utc>,
    pub enable: bool,
}

impl SchoolSignConfig {
    pub fn new(
        id: Uuid,
        owner_user_id: Uuid,
        student_id: String,
        school_task_id: String,
        point: GeoPoint,
        jitter_radius_min_meters: f64,
        jitter_radius_max_meters: f64,
        accuracy_min_meters: f64,
        accuracy_max_meters: f64,
        allow_sign_timerange_start: DateTime<Utc>,
        allow_sign_timerange_end: DateTime<Utc>,
        enable: bool,
    ) -> Result<Self, DomainError> {
        if student_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolUserId);
        }
        if school_task_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolTaskId);
        }
        if !jitter_radius_min_meters.is_finite() || jitter_radius_min_meters < 0.0 {
            return Err(DomainError::InvalidLocationAccuracy(
                jitter_radius_min_meters,
            ));
        }
        if !jitter_radius_max_meters.is_finite() || jitter_radius_max_meters < 0.0 {
            return Err(DomainError::InvalidLocationAccuracy(
                jitter_radius_max_meters,
            ));
        }
        if jitter_radius_min_meters > jitter_radius_max_meters {
            return Err(DomainError::InvalidLocationAccuracy(
                jitter_radius_min_meters,
            ));
        }
        if !accuracy_min_meters.is_finite() || accuracy_min_meters <= 0.0 {
            return Err(DomainError::InvalidLocationAccuracy(accuracy_min_meters));
        }
        if !accuracy_max_meters.is_finite() || accuracy_max_meters <= 0.0 {
            return Err(DomainError::InvalidLocationAccuracy(accuracy_max_meters));
        }
        if accuracy_min_meters > accuracy_max_meters {
            return Err(DomainError::InvalidLocationAccuracy(accuracy_min_meters));
        }
        if allow_sign_timerange_end < allow_sign_timerange_start {
            return Err(DomainError::InvalidDateRange);
        }

        Ok(Self {
            id,
            owner_user_id,
            student_id,
            school_task_id,
            point,
            jitter_radius_min_meters,
            jitter_radius_max_meters,
            accuracy_min_meters,
            accuracy_max_meters,
            allow_sign_timerange_start,
            allow_sign_timerange_end,
            enable,
        })
    }

    pub fn is_allowed_at(&self, utc_now: DateTime<Utc>) -> bool {
        utc_now >= self.allow_sign_timerange_start && utc_now <= self.allow_sign_timerange_end
    }

    /// 由静态配置 + 当前时间，生成领域签到命令。
    pub fn build_checkin_command(
        &self,
        utc_now: DateTime<Utc>,
        noise_generator: &dyn CheckinNoiseGenerator,
    ) -> Result<CheckinCommand, DomainError> {
        if !self.enable {
            return Err(DomainError::SignConfigDisabled);
        }
        if !self.is_allowed_at(utc_now) {
            return Err(DomainError::NotRunnableNow);
        }

        let sampled_accuracy =
            noise_generator.sample_accuracy(self.accuracy_min_meters, self.accuracy_max_meters);

        let sampled_point = noise_generator.sample_point(
            self.point,
            self.jitter_radius_min_meters,
            self.jitter_radius_max_meters,
        );
        CheckinCommand::new(
            &self.school_task_id,
            &sampled_point,
            sampled_accuracy,
            utc_now,
        )
    }
}
