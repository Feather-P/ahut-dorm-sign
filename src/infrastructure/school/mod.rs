pub mod week_mapper;

use async_trait::async_trait;

use crate::domain::{
    error::DomainError,
    school::{
        config::SchoolSignConfig,
        repository::{SchoolSignConfigRepository, SchoolUserRepository},
        user::SchoolUser,
    },
};
