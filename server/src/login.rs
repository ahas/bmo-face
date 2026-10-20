//! The editor's login: a name and password checked against BMO's `web_users`, and everything after
//! that — the session, its cookie, the throttle — this server's own (docs/face.md, 「How a screen
//! signs in」).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use cookie::Cookie;
use rand::RngCore;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

use crate::accounts::Accounts;

pub const COOKIE: &str = "bmo_face";
pub const SESSION_MS: u64 = 7 * 86_400_000;
/// How long a run of failed logins from one address counts against it, and how many it may make.
pub const ATTEMPT_WINDOW_MS: u64 = 300_000;
pub const ATTEMPT_LIMIT: u32 = 20;

#[derive(Debug, PartialEq)]
pub enum LoginResult {
  Signed { token: String, name: String },
  Refused { status: u16, error: &'static str },
}

struct Attempt {
  count: u32,
  since: u64,
}

pub struct EditorLogin {
  db: SqlitePool,
  accounts: Accounts,
  attempts: Mutex<HashMap<String, Attempt>>,
  now: Box<dyn Fn() -> u64 + Send + Sync>,
}

fn hashed(token: &str) -> String {
  format!("{:x}", Sha256::digest(token.as_bytes()))
}

/// The session token a `Cookie` header carries, if any.
fn token_of(cookie_header: Option<&str>) -> Option<String> {
  Cookie::split_parse(cookie_header?)
    .flatten()
    .find(|cookie| cookie.name() == COOKIE)
    .map(|cookie| cookie.value().to_owned())
    .filter(|token| !token.is_empty())
}

pub fn session_cookie(token: &str, secure: bool) -> String {
  format!(
    "{COOKIE}={token}; Path=/bmo-face; HttpOnly; SameSite=Strict; Max-Age={}{}",
    SESSION_MS / 1000,
    if secure { "; Secure" } else { "" }
  )
}

pub fn cleared_cookie() -> String {
  format!("{COOKIE}=; Path=/bmo-face; HttpOnly; SameSite=Strict; Max-Age=0")
}

impl EditorLogin {
  pub async fn open(
    root: &Path,
    accounts: Accounts,
    now: impl Fn() -> u64 + Send + Sync + 'static,
  ) -> Result<EditorLogin, sqlx::Error> {
    let options = SqliteConnectOptions::new()
      .filename(root.join("face.db"))
      .create_if_missing(true)
      .journal_mode(SqliteJournalMode::Wal);
    let db = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS sessions (token TEXT PRIMARY KEY, name TEXT NOT NULL, expires INTEGER NOT NULL)")
      .execute(&db)
      .await?;
    Ok(EditorLogin {
      db,
      accounts,
      attempts: Mutex::new(HashMap::new()),
      now: Box::new(now),
    })
  }

  /// Who the cookie belongs to, or none for no live session.
  pub async fn session_of(&self, cookie_header: Option<&str>) -> Option<String> {
    let token = token_of(cookie_header)?;
    let row = sqlx::query("SELECT name FROM sessions WHERE token = ? AND expires > ?")
      .bind(hashed(&token))
      .bind((self.now)() as i64)
      .fetch_optional(&self.db)
      .await
      .ok()??;
    row.try_get("name").ok()
  }

  pub async fn login(&self, address: &str, body: &Value) -> LoginResult {
    let at = (self.now)();
    let held_off = {
      let mut attempts = self.attempts.lock().unwrap();
      attempts.retain(|_, entry| at - entry.since <= ATTEMPT_WINDOW_MS);
      let attempt = attempts.entry(address.to_owned()).or_insert(Attempt {
        count: 0,
        since: at,
      });
      attempt.count += 1;
      attempt.count > ATTEMPT_LIMIT
    };
    if held_off {
      return LoginResult::Refused {
        status: 429,
        error: "잠시 뒤 다시 로그인하세요.",
      };
    }

    let (Some(name), Some(password)) =
      (body.get("name").and_then(Value::as_str), body.get("password").and_then(Value::as_str))
    else {
      return LoginResult::Refused {
        status: 400,
        error: "로그인 정보를 확인하세요.",
      };
    };
    // Lengths as the screen counts them, in UTF-16 code units.
    if name.encode_utf16().count() > 100 || password.encode_utf16().count() > 1000 {
      return LoginResult::Refused {
        status: 400,
        error: "로그인 정보를 확인하세요.",
      };
    }

    let stored = self.accounts.password_of(name).await.ok().flatten();
    let password = password.to_owned();
    let verified = match stored {
      Some(stored) => tokio::task::spawn_blocking(move || {
        PasswordHash::new(&stored)
          .is_ok_and(|hash| Argon2::default().verify_password(password.as_bytes(), &hash).is_ok())
      })
      .await
      .unwrap_or(false),
      None => false,
    };
    if !verified {
      return LoginResult::Refused {
        status: 401,
        error: "아이디 또는 비밀번호가 맞지 않습니다.",
      };
    }

    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let token: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    let _ = sqlx::query("DELETE FROM sessions WHERE expires < ?")
      .bind(at as i64)
      .execute(&self.db)
      .await;
    let _ = sqlx::query("INSERT INTO sessions VALUES (?, ?, ?)")
      .bind(hashed(&token))
      .bind(name)
      .bind((at + SESSION_MS) as i64)
      .execute(&self.db)
      .await;
    self.attempts.lock().unwrap().remove(address);
    LoginResult::Signed {
      token,
      name: name.to_owned(),
    }
  }

  pub async fn logout(&self, cookie_header: Option<&str>) {
    if let Some(token) = token_of(cookie_header) {
      let _ = sqlx::query("DELETE FROM sessions WHERE token = ?")
        .bind(hashed(&token))
        .execute(&self.db)
        .await;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_session_is_read_off_its_own_cookie_among_others() {
    assert_eq!(token_of(Some("theme=dark; bmo_face=abc123; other=1")), Some("abc123".into()));
    assert_eq!(token_of(Some("bmo_face=")), None);
    assert_eq!(token_of(Some("not_bmo_face=x")), None);
    assert_eq!(token_of(None), None);
  }

  #[test]
  fn the_cookie_is_the_servers_own_under_its_prefix() {
    assert_eq!(
      session_cookie("t", false),
      "bmo_face=t; Path=/bmo-face; HttpOnly; SameSite=Strict; Max-Age=604800"
    );
    assert!(session_cookie("t", true).ends_with("; Secure"));
    assert_eq!(cleared_cookie(), "bmo_face=; Path=/bmo-face; HttpOnly; SameSite=Strict; Max-Age=0");
  }
}
