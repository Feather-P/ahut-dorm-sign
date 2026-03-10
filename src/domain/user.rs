use uuid::Uuid;

pub struct SystemUser {
    id: Uuid,
    username: String,
    role: UserRole,
    time_zone: chrono_tz::Tz,
}

pub enum UserRole {
    Admin,
    User
}