-- 学校相关表

CREATE TABLE IF NOT EXISTS school_users (
    student_id TEXT NOT NULL,
    owner_user_id UUID NOT NULL,
    user_name TEXT NOT NULL,
    credential_storage TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (student_id, owner_user_id)
);

CREATE TABLE IF NOT EXISTS school_sign_configs (
    id UUID NOT NULL,
    owner_user_id UUID NOT NULL,
    student_id TEXT NOT NULL,
    school_task_id TEXT NOT NULL,
    lng DOUBLE PRECISION NOT NULL,
    lat DOUBLE PRECISION NOT NULL,
    jitter_radius_min_meters DOUBLE PRECISION NOT NULL,
    jitter_radius_max_meters DOUBLE PRECISION NOT NULL,
    accuracy_min_meters DOUBLE PRECISION NOT NULL,
    accuracy_max_meters DOUBLE PRECISION NOT NULL,
    allow_sign_timerange_start TIMESTAMPTZ NOT NULL,
    allow_sign_timerange_end TIMESTAMPTZ NOT NULL,
    enable BOOLEAN NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id),
    UNIQUE (student_id, school_task_id)
);

CREATE TABLE IF NOT EXISTS school_sign_tasks (
    id UUID NOT NULL,
    student_id TEXT NOT NULL,
    school_task_id TEXT NOT NULL,
    title TEXT NOT NULL,
    date_start DATE NOT NULL,
    date_end DATE NOT NULL,
    time_start TIME NOT NULL,
    time_end TIME NOT NULL,
    days_of_week TEXT NOT NULL,
    time_zone TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id),
    UNIQUE (student_id, school_task_id)
);

CREATE TABLE IF NOT EXISTS school_sessions (
    owner_user_id UUID NOT NULL,
    student_id TEXT NOT NULL,
    access_token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    expired_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (owner_user_id, student_id)
);

CREATE TABLE IF NOT EXISTS school_user_custom_user_agents (
    id UUID NOT NULL,
    owner_user_id UUID NOT NULL,
    student_id TEXT NOT NULL,
    user_agent TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id),
    UNIQUE (owner_user_id, student_id, user_agent)
);

CREATE INDEX IF NOT EXISTS idx_school_sign_configs_student_enable
    ON school_sign_configs (student_id, enable);

CREATE INDEX IF NOT EXISTS idx_school_sign_tasks_student
    ON school_sign_tasks (student_id);

CREATE INDEX IF NOT EXISTS idx_school_sessions_expired_at
    ON school_sessions (expired_at);

CREATE INDEX IF NOT EXISTS idx_school_user_custom_user_agents_owner_student
    ON school_user_custom_user_agents (owner_user_id, student_id);


-- 用户表
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    user_name TEXT NOT NULL,
    time_zone TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_system_users_username_not_blank
    CHECK (length(btrim(user_name)) > 0)
);

-- 角色表
CREATE TABLE IF NOT EXISTS roles (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_roles_rolename_not_blank
    CHECK (length(btrim(name)) > 0)
)

-- 用户-角色绑定表
CREATE TABLE IF NOT EXISTS user_roles (
    user_id UUID NOT NULL,
    role_code TEXT NOT NULL,
    PRIMARY KEY (user_id, role_code)
)

-- 角色-权限绑定
CREATE TABLE IF NOT EXISTS role_permissions (
    role_code TEXT NOT NULL,
    permission_code TEXT NOT NULL
    PRIMARY KEY (role_code, permission_code)
)