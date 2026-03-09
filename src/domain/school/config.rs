use chrono::Datelike;
use uuid::Uuid;

use crate::domain::{
    error::DomainError,
    school::{
        location::GeoPoint,
        task::{CheckinCommand, CheckinRuntime},
    },
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
        enable: bool,
    ) -> Result<Self, DomainError> {
        if student_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolUserId);
        }
        if school_task_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolTaskId);
        }
        if !jitter_radius_min_meters.is_finite() || jitter_radius_min_meters < 0.0 {
            return Err(DomainError::InvalidLocationAccuracy(jitter_radius_min_meters));
        }
        if !jitter_radius_max_meters.is_finite() || jitter_radius_max_meters < 0.0 {
            return Err(DomainError::InvalidLocationAccuracy(jitter_radius_max_meters));
        }
        if jitter_radius_min_meters > jitter_radius_max_meters {
            return Err(DomainError::InvalidLocationAccuracy(jitter_radius_min_meters));
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
            enable,
        })
    }

    /// 由静态配置 + 运行时上下文，生成领域签到命令。
    pub fn build_checkin_command(
        &self,
        runtime: &CheckinRuntime,
        point: GeoPoint,
        accuracy_meters: f64,
    ) -> Result<CheckinCommand, DomainError> {
        if !self.enable {
            return Err(DomainError::SignConfigDisabled);
        }
        if !accuracy_meters.is_finite() || accuracy_meters <= 0.0 {
            return Err(DomainError::InvalidLocationAccuracy(accuracy_meters));
        }
        if accuracy_meters < self.accuracy_min_meters || accuracy_meters > self.accuracy_max_meters {
            return Err(DomainError::InvalidLocationAccuracy(accuracy_meters));
        }
        self.ensure_point_within_jitter(point)?;

        let local = runtime.local_now;
        Ok(CheckinCommand {
            task_id: self.school_task_id.clone(),
            point,
            accuracy_meters,
            date: local.date_naive(),
            time: local.time(),
            weekday: local.weekday(),
        })
    }

    fn ensure_point_within_jitter(&self, point: GeoPoint) -> Result<(), DomainError> {
        let base_lat = self.point.lat();
        let base_lng = self.point.lng();
        let target_lat = point.lat();
        let target_lng = point.lng();

        let delta_lat_m = (target_lat - base_lat) * 111_320.0;
        let delta_lng_m = (target_lng - base_lng) * 111_320.0 * base_lat.to_radians().cos();
        let distance_m = (delta_lat_m.powi(2) + delta_lng_m.powi(2)).sqrt();

        if distance_m < self.jitter_radius_min_meters || distance_m > self.jitter_radius_max_meters {
            return Err(DomainError::InvalidCoordinates(target_lng, target_lat));
        }

        Ok(())
    }
}
