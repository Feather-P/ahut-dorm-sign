use rand::Rng;

/// 在 [min, max] 区间内随机生成一个保留两位小数（四舍五入）的 f64。
///
/// 返回值示例：12.34
pub fn random_with_range(min: u32, max: u32) -> f64 {
    let (low, high) = if min <= max { (min, max) } else { (max, min) };

    let mut rng = rand::thread_rng();
    let value = rng.gen_range(low as f64..=high as f64);

    (value * 100.0).round() / 100.0
}
