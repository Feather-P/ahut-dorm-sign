use crate::error::ServiceError;

/// 通用业务响应包裹。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BizEnvelope<T> {
    pub code: i32,
    pub success: bool,
    pub msg: String,
    pub data: T,
}

impl<T> BizEnvelope<T> {
    /// 校验业务包裹并提取 data。
    pub fn into_data(self, service: &'static str) -> Result<T, ServiceError> {
        if self.success && self.code == 200 {
            Ok(self.data)
        } else {
            Err(ServiceError::RemoteBusiness {
                service,
                code: self.code,
                msg: self.msg,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BizEnvelope;

    #[test]
    fn should_extract_data_when_success_and_code_200() {
        let envelope = BizEnvelope {
            code: 200,
            success: true,
            msg: "ok".to_string(),
            data: 42,
        };

        let data = envelope.into_data("test.service").expect("应成功提取 data");
        assert_eq!(data, 42);
    }

    #[test]
    fn should_return_remote_business_when_success_false() {
        let envelope = BizEnvelope {
            code: 200,
            success: false,
            msg: "biz failed".to_string(),
            data: (),
        };

        let err = envelope
            .into_data("test.service")
            .expect_err("应返回 RemoteBusiness");

        let text = err.to_string();
        assert!(text.contains("test.service"));
        assert!(text.contains("200"));
        assert!(text.contains("biz failed"));
    }

    #[test]
    fn should_return_remote_business_when_code_not_200() {
        let envelope = BizEnvelope {
            code: 500,
            success: true,
            msg: "server error".to_string(),
            data: (),
        };

        let err = envelope
            .into_data("test.service")
            .expect_err("应返回 RemoteBusiness");

        let text = err.to_string();
        assert!(text.contains("test.service"));
        assert!(text.contains("500"));
        assert!(text.contains("server error"));
    }
}
