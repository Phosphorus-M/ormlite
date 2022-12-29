use std::env::var;
use std::fs;
use std::io::Read;
use std::path::Path;
use anyhow::{Error, Result};
use clap::Parser;
use sqlx::postgres::PgArguments;
use ormlite::{Acquire, Executor};
use crate::command::{get_executed_migrations, get_pending_migrations, MigrationType};
use crate::config::{get_var_backup_folder, get_var_database_url, get_var_migration_folder};
use crate::util::{CommandSuccess, create_connection, create_runtime};
use sqlx::Arguments;
use url::Url;

#[derive(Parser, Debug)]
pub struct Down {
    target: Option<String>,

    #[clap(long, short)]
    /// By default, the `down` command will perform a dry run. Use -f to run it.
    force: bool,
}

const CLEAR_DATABASE_QUERY: &str = "DROP SCHEMA public CASCADE;
CREATE SCHEMA public;
GRANT ALL ON SCHEMA public TO $USER;
GRANT ALL ON SCHEMA public TO public;
";

fn get_backups(backup_folder: &Path) -> Result<Vec<String>> {
    let mut backups = std::fs::read_dir(backup_folder)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name().into_string().ok()?;
            if file_name.ends_with(".sql.bak") {
                Some(file_name)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    backups.sort();
    Ok(backups)
}

impl Down {
    pub fn run(self) -> Result<()> {
        let folder = get_var_migration_folder();
        let runtime = create_runtime();
        let url = get_var_database_url();
        let mut conn_owned = create_connection(&url, &runtime)?;
        let mut conn = runtime.block_on(conn_owned.acquire())?;

        let mut executed = get_executed_migrations(&runtime, conn)?;
        let pending = get_pending_migrations(&folder)?
            .into_iter()
            .filter(|m| m.migration_type() != MigrationType::Up)
            .collect::<Vec<_>>();

        let Some(last_pending) = pending.last() else {
            return Err(Error::msg("No migrations were found in the migrations folder."));
        };

        if last_pending.migration_type() == MigrationType::Simple {
            let target = if let Some(target) = self.target {
                target
            } else if executed.len() > 1 {
                executed.iter().skip(1).next().unwrap().name.clone()
            } else if executed.len() == 1 {
                "0_empty".to_string()
            } else {
                return Err(Error::msg("No target migration was specified and there are no migrations to rollback to."));
            };

            let backup_folder = get_var_backup_folder();
            let backups = get_backups(&backup_folder)?;
            let Some(backup) = backups.iter().find(|b| {
                if target.contains('_') {
                    **b == target
                } else {
                    b.starts_with(&format!("{}_", target))
                }
            }) else {
                return Err(Error::msg(format!("Looked for snapshot `{}` in {}, but could not find it.", target, backup_folder.display())));
            };

            if !self.force {
                println!("Re-run with -f to execute rollback. This command will restore the following snapshot:\n{}", backup_folder.join(&backup).display());
                return Ok(())
            }

            let mut user = Url::parse(&url)?.username().to_string();
            if user.is_empty() {
                user = var("USER")?
            }

            runtime.block_on(conn.execute(&*CLEAR_DATABASE_QUERY.replace("$USER", &user)))?;
            let restore_file = fs::File::open(backup_folder.join(&backup))?;
            std::process::Command::new("psql")
                .arg(url)
                .arg("-q")
                .stdin(restore_file)
                .ok_or("Failed to restore database.")?;
        } else {
            executed.reverse();
            if let Some(target) = self.target {
                executed = executed.into_iter().take_while(|m| {
                    let matches = if target.chars().all(|c| c.is_numeric()) {
                        m.version_str() == target
                    } else if target.contains('_') {
                        m.name == target
                    } else {
                        m.description == target
                    };
                    !matches
                }).collect();
            } else {
                executed.truncate(1);
            }
            if !self.force {
                println!("Re-run with -f to execute rollbacks. This command will run the following rollbacks:");
            }
            for migration in executed {
                let file_path = folder.join(migration.name).with_extension("down.sql");
                if !self.force {
                    println!("{}", file_path.display());
                } else {
                    let body = fs::read_to_string(&file_path)?;
                    // hack that sqlx screws up lifetimes and we have to acquire for each for loop
                    let mut conn = runtime.block_on(conn_owned.acquire())?;
                    runtime.block_on(conn.execute(&*body))?;
                    let mut args = PgArguments::default();
                    args.add(migration.version);
                    let q = sqlx::query_with("DELETE FROM _sqlx_migrations WHERE version = $1", args);
                    runtime.block_on(q.execute(conn))?;
                }
            }
        }
        Ok(())
    }
}