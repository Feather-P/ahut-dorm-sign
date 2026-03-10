use uuid::Uuid;

use crate::domain::{
    error::DomainError,
    school::{
        location::GeoPoint,
        noise::CheckinNoiseGenerator,
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
        noise_generator: &dyn CheckinNoiseGenerator,
    ) -> Result<CheckinCommand, DomainError> {
        if !self.enable {
            return Err(DomainError::SignConfigDisabled);
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
            runtime.utc_now,
        )
    }
}
