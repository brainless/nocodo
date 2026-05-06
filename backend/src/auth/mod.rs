pub mod handlers;
pub mod middleware;
pub mod types;

use rusqlite::{params, Connection, OptionalExtension};

pub struct AuthConfig {
    pub db_url: String,
    pub mandatory: bool,
    pub resend_api_key: Option<String>,
    pub from_email: Option<String>,
}

pub fn generate_otp() -> String {
    use rand::Rng;
    format!("{:06}", rand::thread_rng().gen_range(0..1_000_000))
}

pub fn generate_session_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn store_otp(conn: &Connection, email: &str, otp: &str) -> rusqlite::Result<()> {
    let expires_at = unix_now() + 30 * 60;
    conn.execute(
        "INSERT OR REPLACE INTO auth_otps (email, otp, expires_at) VALUES (?1, ?2, ?3)",
        params![email, otp, expires_at],
    )?;
    Ok(())
}

pub fn verify_and_consume_otp(
    conn: &Connection,
    email: &str,
    otp: &str,
) -> rusqlite::Result<bool> {
    let now = unix_now();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM auth_otps WHERE email = ?1 AND otp = ?2 AND expires_at > ?3",
        params![email, otp, now],
        |row| row.get(0),
    )?;
    if count > 0 {
        conn.execute("DELETE FROM auth_otps WHERE email = ?1", params![email])?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn create_session(conn: &Connection, email: &str) -> rusqlite::Result<String> {
    let token = generate_session_token();
    let expires_at = unix_now() + 24 * 60 * 60;
    conn.execute(
        "INSERT INTO auth_sessions (token, email, expires_at) VALUES (?1, ?2, ?3)",
        params![token, email, expires_at],
    )?;
    Ok(token)
}

pub fn validate_session_sync(db_url: &str, token: &str) -> bool {
    let Ok(conn) = Connection::open(db_url) else {
        return false;
    };
    let now = unix_now();
    conn.query_row(
        "SELECT COUNT(*) FROM auth_sessions WHERE token = ?1 AND expires_at > ?2",
        params![token, now],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count > 0)
    .unwrap_or(false)
}

pub fn delete_session(conn: &Connection, token: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM auth_sessions WHERE token = ?1", params![token])?;
    Ok(())
}

pub fn get_session_email(conn: &Connection, token: &str) -> rusqlite::Result<Option<String>> {
    let now = unix_now();
    conn.query_row(
        "SELECT email FROM auth_sessions WHERE token = ?1 AND expires_at > ?2",
        params![token, now],
        |row| row.get(0),
    )
    .optional()
}

pub async fn send_otp_email(
    api_key: &str,
    from_email: &str,
    to_email: &str,
    otp: &str,
) -> Result<(), String> {
    use resend_rs::types::CreateEmailBaseOptions;
    use resend_rs::Resend;

    let client = Resend::new(api_key);
    let html = format!(
        "<p>Your verification code is: <strong style=\"font-size:1.5em;letter-spacing:0.1em\">{}</strong></p>\
         <p>This code expires in 30 minutes.</p>",
        otp
    );
    let email = CreateEmailBaseOptions::new(from_email, [to_email], "Your sign-in code").with_html(&html);
    client
        .emails
        .send(email)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
