use crate::domain::school::location::GeoPoint;

pub trait CheckinNoiseGenerator: Send + Sync {
    fn sample_point(
        &self,
        base: GeoPoint,
        min_radius_meters: f64,
        max_radius_meters: f64,
    ) -> GeoPoint;

    fn sample_accuracy(&self, min_accuracy_meters: f64, max_accuracy_meters: f64) -> f64;
}
