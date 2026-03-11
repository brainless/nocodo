#[cfg(not(any(feature = "db-sqlite", feature = "db-postgres")))]
compile_error!("Enable one DB feature: db-sqlite or db-postgres");

#[cfg(all(feature = "db-sqlite", feature = "db-postgres"))]
compile_error!("Enable only one DB feature: db-sqlite or db-postgres");

use std::io;

#[cfg(feature = "db-sqlite")]
mod sqlite {
    use super::*;
    use refinery::embed_migrations;
    use rusqlite::Connection;

    embed_migrations!("migrations/sqlite");

    pub fn run_startup_migrations(database_url: &str) -> io::Result<()> {
        let mut conn = Connection::open(database_url).map_err(io::Error::other)?;
        migrations::runner().run(&mut conn).map_err(io::Error::other)?;
        Ok(())
    }
}

#[cfg(feature = "db-postgres")]
mod postgres_db {
    use super::*;
    use postgres::{Client, NoTls};
    use refinery::embed_migrations;

    embed_migrations!("migrations/postgres");

    pub fn run_startup_migrations(database_url: &str) -> io::Result<()> {
        let mut client = Client::connect(database_url, NoTls).map_err(io::Error::other)?;
        migrations::runner().run(&mut client).map_err(io::Error::other)?;
        Ok(())
    }
}

#[cfg(feature = "db-sqlite")]
pub use sqlite::run_startup_migrations;

#[cfg(feature = "db-postgres")]
pub use postgres_db::run_startup_migrations;
