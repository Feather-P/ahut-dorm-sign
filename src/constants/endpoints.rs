/// 你工学生考勤系统api基底url
pub const BASE_URL: &str = "https://xskq.ahut.edu.cn/api";

/// 你工考勤系统登录路径
pub const LOGIN: &str = "/flySource-auth/oauth/token";
/// 你工考勤系统获取列表路径
pub const DORM_LIST: &str = "/flySource-yxgl/dormSignTask/getListForApp";
/// 你工考勤系统微信 JS SDK 配置记录路径
pub const DORM_WECHAT_MP_CONFIG: &str = "/flySource-base/wechat/getWechatMpConfig";
/// 你工考勤系统页面访问日志记录路径
pub const DORM_API_LOG_SAVE: &str = "/flySource-base/apiLog/save";
