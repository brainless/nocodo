CREATE TABLE IF NOT EXISTS auth_otps (
    email      TEXT PRIMARY KEY,
    otp        TEXT NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS auth_sessions (
    token      TEXT PRIMARY KEY,
    email      TEXT NOT NULL,
    expires_at INTEGER NOT NULL
);
