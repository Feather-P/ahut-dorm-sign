use chrono::{Duration, NaiveDate};
use crate::utils::serde_time::{date_from_datetime_without_tz, duration_seconds};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthInfo {
    /// 服务器返回的token，后续api访问用的也是这个
    pub access_token: String,
    /// token类型，目前只观察到 "bearer" 一种
    pub token_type: String,
    /// 服务器返回的刷新token，目前没观察到在哪有用
    pub refresh_token: String,
    /// 服务器返回的token过期时间，原数据为秒
    #[serde(with = "duration_seconds")]
    pub expires_in: Duration,
    /// 服务器返回的token作用域信息，目前只观察到 "all" 一种
    pub scope: String,
    /// 服务器返回的账号密码等级
    #[serde(rename = "passWordLevel")]
    pub password_level: i32,
    /// 服务器返回的头像url
    #[serde(rename = "avatarUrl")]
    pub avatar_url: String,
    /// 服务器返回的账号类型，学生应为 5
    #[serde(rename = "accountType")]
    pub account_type: i32,
    /// 服务器返回的用户姓名，应为实际证件姓名
    #[serde(rename = "userName")]
    pub user_name: String,
    /// 服务器返回的用户角色类型
    #[serde(rename = "roleType")]
    pub role_type: String,
    /// 服务器返回的用户id，应为学号
    #[serde(rename = "userId")]
    pub user_id: String,
    /// 服务器返回的上次登录日期（从 "YYYY-MM-DD HH:MM:SS" 中提取日期部分）
    #[serde(rename = "lastLoginTime", deserialize_with = "date_from_datetime_without_tz::deserialize")]
    pub last_login_time: NaiveDate,
    /// oauth标准id，不知道干什么的，测试返回为空
    #[serde(rename = "oauthId")]
    pub oauth_id: Option<String>,
    /// 服务器返回的账号编号，应为学号
    #[serde(rename = "accountNo")]
    pub account_no: String,
    /// 服务器返回的校区id，目前只观察到 "000000"
    #[serde(rename = "tenantId")]
    pub tenant_id: String,
    /// 服务器返回的用户角色名称
    #[serde(rename = "roleName")]
    pub role_name: String,
    /// 服务器返回的用户类型，一般学生账号对应为 2
    #[serde(rename = "userType")]
    pub user_type: i32,
    /// 账号细节，包含二步验证方式
    pub detail: AuthDetail,
    /// 服务器返回的学校名称
    #[serde(rename = "schoolName")]
    pub school_name: String,
    /// json web token的JTI
    pub jti: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthDetail {
    #[serde(rename = "sysAuthType")]
    pub sys_auth_type: String,
    #[serde(rename = "isSysUserSecondAuth")]
    pub is_sys_user_second_auth: bool,
}
