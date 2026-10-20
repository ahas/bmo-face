//! BMO's web accounts, which the editor logs in against: a name's password hash, read from BMO's
//! archive and never written. The archive stands where BMO's own setting says: the address
//! `BMO_ARCHIVE_URL` holds, the file where that is set empty, and for BMO's own `bmo.db` the address
//! its data root's `env/archive-url` holds, where it holds one.

use std::path::{Path, PathBuf};

use sqlx::any::{AnyPoolOptions, install_default_drivers};
use sqlx::{AnyPool, Row};

const PASSWORD_SQL: &str = "SELECT password_hash AS password FROM web_users WHERE name = $1";

/// Where BMO's archive stands for this process: a PostgreSQL address, or a file.
pub fn archive_place(env: impl Fn(&str) -> Option<String>, home: &Path) -> String {
  let bmo_data = home.join(".local/share/bmo/data");
  let file = env("BMO_DB_PATH").map(PathBuf::from).unwrap_or_else(|| bmo_data.join("bmo.db"));
  let address = match env("BMO_ARCHIVE_URL") {
    Some(named) => named.trim().to_owned(),
    None if std::path::absolute(&file).ok() == Some(bmo_data.join("bmo.db")) => {
      std::fs::read_to_string(bmo_data.join("env/archive-url"))
        .map(|text| text.trim().to_owned())
        .unwrap_or_default()
    }
    None => String::new(),
  };
  if address.is_empty() { file.to_string_lossy().into_owned() } else { address }
}

/// BMO's accounts wherever its archive stands, reached when a login first asks.
pub struct Accounts {
  pool: AnyPool,
}

impl Accounts {
  pub fn at(place: &str) -> Result<Accounts, sqlx::Error> {
    install_default_drivers();
    let address = if place.starts_with("postgres://") || place.starts_with("postgresql://") {
      place.to_owned()
    } else {
      format!("sqlite://{place}?mode=ro")
    };
    Ok(Accounts {
      pool: AnyPoolOptions::new().max_connections(1).connect_lazy(&address)?,
    })
  }

  /// A name's password hash, or none where BMO has no account of that name.
  pub async fn password_of(&self, name: &str) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query(PASSWORD_SQL).bind(name).fetch_optional(&self.pool).await?;
    row.map(|row| row.try_get::<String, _>("password")).transpose()
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::*;

  #[test]
  fn the_archive_stands_where_bmos_own_setting_says() {
    let home = tempfile::tempdir().unwrap();
    let data = home.path().join(".local/share/bmo/data");
    std::fs::create_dir_all(data.join("env")).unwrap();
    let place = |pairs: &[(&str, &str)]| {
      let env: HashMap<String, String> =
        pairs.iter().map(|(key, value)| ((*key).into(), (*value).into())).collect();
      archive_place(|key| env.get(key).cloned(), home.path())
    };
    let own = data.join("bmo.db").to_string_lossy().into_owned();

    assert_eq!(place(&[]), own);
    std::fs::write(data.join("env/archive-url"), "postgres:///bmo?host=/run/postgresql\n").unwrap();
    assert_eq!(place(&[]), "postgres:///bmo?host=/run/postgresql");
    assert_eq!(place(&[("BMO_DB_PATH", "/copy/bmo.db")]), "/copy/bmo.db");
    assert_eq!(place(&[("BMO_ARCHIVE_URL", "")]), own);
    assert_eq!(place(&[("BMO_ARCHIVE_URL", " postgres:///other ")]), "postgres:///other");
  }
}
