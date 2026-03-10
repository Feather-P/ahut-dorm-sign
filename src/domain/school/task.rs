use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc, WeekdaySet};
use uuid::Uuid;

use crate::domain::{error::DomainError, school::location::GeoPoint};

pub struct SchoolSignTask {
    id: Uuid,
    student_id: String,
    school_task_id: String,
    title: String,
    date_range: DateRange,
    daily_time_window: TimeWindow,
    days_of_week: WeekdaySet,
    time_zone: chrono_tz::Tz,
}

impl SchoolSignTask {
    pub fn new(
        id: Uuid,
        student_id: String,
        school_task_id: String,
        title: String,
        date_range: DateRange,
        daily_time_window: TimeWindow,
        days_of_week: WeekdaySet,
        time_zone: chrono_tz::Tz,
    ) -> Result<Self, DomainError> {
        if school_task_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolTaskId);
        }
        if title.trim().is_empty() {
            return Err(DomainError::BlankTitle);
        }
        if days_of_week.is_empty() {
            return Err(DomainError::BlankDaysOfWeek);
        }

        Ok(Self {
            id,
            student_id,
            school_task_id,
            title,
            date_range,
            daily_time_window,
            days_of_week,
            time_zone,
        })
    }

    /// 判断是否符合签到条件
    pub fn is_runnable_at(&self, utc_now: DateTime<chrono::Utc>) -> bool {
        let local_now = utc_now.with_timezone(&self.time_zone);

        let date = local_now.date_naive();
        let time = local_now.time();
        let weekday = local_now.weekday();

        if !self.daily_time_window.is_within_range(time) {
            return false;
        }
        if !self.date_range.is_within_range(date) {
            return false;
        }
        if !self.days_of_week.contains(weekday) {
            return false;
        }

        true
    }

    pub fn school_task_id(&self) -> &str {
        &self.school_task_id
    }

    pub fn time_zone(&self) -> &chrono_tz::Tz {
        &self.time_zone
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeWindow {
    start: NaiveTime,
    end: NaiveTime,
}

impl TimeWindow {
    pub fn new(start: NaiveTime, end: NaiveTime) -> Result<Self, DomainError> {
        if end <= start {
            return Err(DomainError::InvalidTimeWindow);
        }

        Ok(Self { start, end })
    }

    pub fn is_within_range(&self, time_now: NaiveTime) -> bool {
        time_now >= self.start && time_now <= self.end
    }

    pub fn start(&self) -> NaiveTime {
        self.start
    }

    pub fn end(&self) -> NaiveTime {
        self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl DateRange {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Result<Self, DomainError> {
        if end < start {
            return Err(DomainError::InvalidDateRange);
        }

        Ok(Self { start, end })
    }

    pub fn is_within_range(&self, date_now: NaiveDate) -> bool {
        date_now >= self.start && date_now <= self.end
    }

    pub fn start(&self) -> NaiveDate {
        self.start
    }

    pub fn end(&self) -> NaiveDate {
        self.end
    }
}

pub struct CheckinCommand {
    task_id: String,
    point: GeoPoint,
    accuracy_meters: f64,
    occurred_at_utc: DateTime<Utc>,
}

impl CheckinCommand {
    pub fn new(
        task_id: &str,
        point: &GeoPoint,
        accuracy_meters: f64,
        occurred_at_utc: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if task_id.trim().is_empty() {
            return Err(DomainError::BlankSchoolTaskId);
        }
        if accuracy_meters.is_infinite() || accuracy_meters <= 0.0 {
            return Err(DomainError::InvalidLocationAccuracy(accuracy_meters))
        }
        Ok(Self {
            task_id: task_id.to_owned(),
            point: point.to_owned(),
            accuracy_meters,
            occurred_at_utc,
        })
    }
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn point(&self) -> GeoPoint {
        self.point
    }

    pub fn accuracy_meters(&self) -> f64 {
        self.accuracy_meters
    }

    pub fn occurred_at_utc(&self) -> DateTime<Utc> {
        self.occurred_at_utc
    }
}

/// 运行时上下文（由应用层注入时钟与任务解析结果）
pub struct CheckinRuntime {
    pub utc_now: DateTime<Utc>,
}

impl CheckinRuntime {
    pub fn new(utc_now: DateTime<Utc>) -> Self {
        Self { utc_now }
    }
}
