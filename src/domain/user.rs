use uuid::Uuid;

pub struct SystemUser {
    id: Uuid,
    username: String,
    role: UserRole
}

pub enum UserRole {
    Admin,
    User
}