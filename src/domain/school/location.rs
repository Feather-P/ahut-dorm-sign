use std::fmt;

use geo_types::{Point, coord};

use crate::domain::error::DomainError;
#[derive(Debug, Clone, Copy, PartialEq)]

pub struct GeoPoint(Point<f64>);

impl GeoPoint {
    /// 创建新的 GeoPoint。遵循 (lng, lat) 即 (x, y) 的顺序。
    pub fn new(lng: f64, lat: f64) -> Result<Self, DomainError> {
        if !(-180.0..=180.0).contains(&lng) || !(-90.0..=90.0).contains(&lat) {
            return Err(DomainError::InvalidCoordinates(lng, lat));
        }
        let c = coord! {
            x: lng,
            y: lat,
        };

        Ok(Self(Point::from(c)))
    }

    /// 解构获取内部原始类型
    pub fn into_inner(self) -> Point<f64> {
        self.0
    }

    /// 经度（x）
    pub fn lng(&self) -> f64 {
        self.0.x()
    }

    /// 纬度（y）
    pub fn lat(&self) -> f64 {
        self.0.y()
    }
}

impl fmt::Display for GeoPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.lng(), self.lat())
    }
}
