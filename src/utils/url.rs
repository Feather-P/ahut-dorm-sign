use reqwest::Url;

/// 拼接基础地址与接口路径，保证只有一个 `/` 分隔。
pub fn join_base_and_path(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{}/{}", base, path)
}

/// 获取用于签名计算的 URL 基础串（去掉 query 后追加 `?sign=`）。
///
/// 支持绝对 URL 与相对路径：
/// - `https://a.com/api/x?y=1` -> `/api/x?sign=`
/// - `/api/x?y=1` -> `/api/x?sign=`
pub fn to_sign_path(url_or_path: &str) -> String {
    if let Ok(parsed) = Url::parse(url_or_path) {
        return format!("{}?sign=", parsed.path());
    }

    let path_without_query = url_or_path.split('?').next().unwrap_or(url_or_path);
    format!("{}?sign=", path_without_query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_join_base_and_path_with_single_slash() {
        assert_eq!(
            join_base_and_path("https://xskq.ahut.edu.cn/api/", "/auth/login"),
            "https://xskq.ahut.edu.cn/api/auth/login"
        );
    }

    #[test]
    fn should_convert_absolute_url_to_sign_path() {
        assert_eq!(
            to_sign_path("https://example.com/api/auth/login?x=1&y=2"),
            "/api/auth/login?sign="
        );
    }

    #[test]
    fn should_convert_relative_path_to_sign_path() {
        assert_eq!(to_sign_path("/api/auth/login?x=1&y=2"), "/api/auth/login?sign=");
    }
}
