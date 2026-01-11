//! Migration file parser
//!
//! Parses SQL migration files with up/down migrations.

use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Timestamp format for migration files
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// Direction of migration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationDirection {
    Up,
    Down,
}

/// A parsed migration with up and down SQL
#[derive(Debug, Clone)]
pub struct Migration {
    /// Unique identifier (e.g., "001_add_status_column")
    pub id: String,
    /// Description from filename
    pub description: String,
    /// SQL to apply the migration
    pub up_sql: String,
    /// SQL to revert the migration
    pub down_sql: String,
    /// Path to the up migration file
    pub up_file: PathBuf,
    /// Path to the down migration file (if exists)
    pub down_file: Option<PathBuf>,
}

impl Migration {
    /// Get SQL for the specified direction
    pub fn get_sql(&self, direction: MigrationDirection) -> &str {
        match direction {
            MigrationDirection::Up => &self.up_sql,
            MigrationDirection::Down => &self.down_sql,
        }
    }
}

/// Parser for migration files
pub struct MigrationParser {
    migrations_dir: PathBuf,
}

impl MigrationParser {
    /// Create a new migration parser
    pub fn new(migrations_dir: PathBuf) -> Self {
        Self { migrations_dir }
    }

    /// Load all migrations from the migrations directory
    pub fn load_all(&self) -> Result<Vec<Migration>> {
        if !self.migrations_dir.exists() {
            return Ok(Vec::new());
        }

        let mut migrations = Vec::new();
        let entries = fs::read_dir(&self.migrations_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "sql") {
                let filename = path.file_stem().unwrap().to_string_lossy();

                // Check if this is an up or down migration
                if filename.ends_with(".up") {
                    let base_name = filename.trim_end_matches(".up");
                    if let Some(migration) = self.load_migration(base_name)? {
                        migrations.push(migration);
                    }
                }
            }
        }

        // Sort migrations by ID
        migrations.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(migrations)
    }

    /// Load a specific migration by its base name
    fn load_migration(&self, base_name: &str) -> Result<Option<Migration>> {
        let up_file = self.migrations_dir.join(format!("{}.up.sql", base_name));
        let down_file = self.migrations_dir.join(format!("{}.down.sql", base_name));

        if !up_file.exists() {
            return Ok(None);
        }

        let up_sql = fs::read_to_string(&up_file)?;
        let down_sql = if down_file.exists() {
            fs::read_to_string(&down_file)?
        } else {
            String::new()
        };

        // Parse ID and description from filename
        // Format: 001_add_status_column
        let parts: Vec<&str> = base_name.splitn(2, '_').collect();
        let id = if parts.len() >= 2 {
            parts[0].to_string()
        } else {
            base_name.to_string()
        };

        let description = if parts.len() >= 2 {
            parts[1].replace('_', " ")
        } else {
            String::new()
        };

        Ok(Some(Migration {
            id,
            description,
            up_sql,
            down_sql,
            up_file,
            down_file: if down_file.exists() {
                Some(down_file)
            } else {
                None
            },
        }))
    }

    /// Create a new migration file pair
    pub fn create_migration(&self, name: &str) -> Result<(PathBuf, PathBuf)> {
        // Create migrations directory if it doesn't exist
        if !self.migrations_dir.exists() {
            fs::create_dir_all(&self.migrations_dir)?;
        }

        // Find the highest existing migration number
        let migrations = self.load_all()?;
        let next_num = migrations
            .iter()
            .filter_map(|m| m.id.parse::<u32>().ok())
            .max()
            .map(|n| n + 1)
            .unwrap_or(1);

        // Create filename with zero-padded number
        let filename = format!("{:03}_{}", next_num, name.replace(' ', "_"));
        let up_file = self.migrations_dir.join(format!("{}.up.sql", filename));
        let down_file = self.migrations_dir.join(format!("{}.down.sql", filename));

        // Create template files
        let up_template = format!(
            "-- Migration: {}\n-- Created: {}\n\n-- Add your up migration SQL here\n",
            name,
            chrono::Utc::now().format(TIMESTAMP_FORMAT)
        );

        let down_template = format!(
            "-- Migration: {}\n-- Created: {}\n\n-- Add your down migration SQL here\n",
            name,
            chrono::Utc::now().format(TIMESTAMP_FORMAT)
        );

        fs::write(&up_file, up_template)?;
        fs::write(&down_file, down_template)?;

        Ok((up_file, down_file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_load_migration() {
        let temp_dir = TempDir::new().unwrap();
        let migrations_dir = temp_dir.path().to_path_buf();

        // Create a test migration
        let up_file = migrations_dir.join("001_add_column.up.sql");
        let down_file = migrations_dir.join("001_add_column.down.sql");

        fs::write(&up_file, "ALTER TABLE users ADD COLUMN status TEXT;").unwrap();
        fs::write(&down_file, "ALTER TABLE users DROP COLUMN status;").unwrap();

        let parser = MigrationParser::new(migrations_dir);
        let migrations = parser.load_all().unwrap();

        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].id, "001");
        assert_eq!(migrations[0].description, "add column");
        assert!(migrations[0].up_sql.contains("ALTER TABLE"));
    }

    #[test]
    fn test_create_migration() {
        let temp_dir = TempDir::new().unwrap();
        let migrations_dir = temp_dir.path().to_path_buf();

        let parser = MigrationParser::new(migrations_dir.clone());
        let (up_file, down_file) = parser.create_migration("add status column").unwrap();

        assert!(up_file.exists());
        assert!(down_file.exists());
        assert!(up_file.to_string_lossy().contains("001_add_status_column"));
    }
}
