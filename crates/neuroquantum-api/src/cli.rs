use crate::auth::AuthService;
use anyhow::{Context, Result};
use base64::Engine;
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::warn;

#[derive(Parser)]
#[command(name = "neuroquantum-api")]
#[command(about = "NeuroQuantumDB API Server", long_about = None)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize `NeuroQuantumDB` with first admin key
    Init {
        /// Name for the admin key
        #[arg(short, long, default_value = "admin")]
        name: String,

        /// Expiry in hours (default: 1 year)
        #[arg(short, long, default_value = "8760")]
        expiry_hours: u32,

        /// Output file for the admin key
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Skip interactive confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Generate a secure JWT secret for configuration
    GenerateJwtSecret {
        /// Output to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Manage API keys (create, list, revoke)
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },

    /// Start the API server (default command)
    Serve,

    /// Manage database migrations
    Migrate {
        #[command(subcommand)]
        action: MigrateAction,
    },

    /// Health check for Docker/Kubernetes
    HealthCheck {
        /// Server URL to check
        #[arg(short, long, default_value = "http://127.0.0.1:8080")]
        url: String,

        /// Timeout in seconds
        #[arg(short, long, default_value = "5")]
        timeout: u64,
    },
}

#[derive(Subcommand)]
pub enum KeyAction {
    /// Create a new API key (requires existing admin key)
    Create {
        /// Name for the new API key
        #[arg(short, long)]
        name: String,

        /// Admin API key for authentication (or set `NEUROQUANTUM_ADMIN_KEY` env var)
        #[arg(long)]
        admin_key: Option<String>,

        /// Permissions (comma-separated: admin,read,write,quantum,neuromorphic,dna)
        #[arg(short, long, value_delimiter = ',')]
        permissions: Vec<String>,

        /// Expiry in hours
        #[arg(short, long, default_value = "720")]
        expiry_hours: u32,

        /// Rate limit per hour
        #[arg(short, long)]
        rate_limit: Option<u32>,

        /// Output file for the new key
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// List all API keys
    List {
        /// Admin API key for authentication (or set `NEUROQUANTUM_ADMIN_KEY` env var)
        #[arg(long)]
        admin_key: Option<String>,
    },

    /// Revoke an API key
    Revoke {
        /// Admin API key for authentication (or set `NEUROQUANTUM_ADMIN_KEY` env var)
        #[arg(long)]
        admin_key: Option<String>,

        /// API key to revoke (prefix match supported)
        #[arg(short, long)]
        key: String,
    },

    /// Show statistics about API keys
    Stats {
        /// Admin API key for authentication (or set `NEUROQUANTUM_ADMIN_KEY` env var)
        #[arg(long)]
        admin_key: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum MigrateAction {
    /// Run pending migrations (up direction)
    Up {
        /// Migrations directory
        #[arg(short, long, default_value = "migrations")]
        dir: PathBuf,

        /// Dry run - don't actually apply changes
        #[arg(long)]
        dry_run: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Rollback last N migrations (down direction)
    Down {
        /// Number of migrations to rollback
        #[arg(short, long, default_value = "1")]
        count: usize,

        /// Migrations directory
        #[arg(short = 'd', long, default_value = "migrations")]
        dir: PathBuf,

        /// Dry run - don't actually apply changes
        #[arg(long)]
        dry_run: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show migration status
    Status {
        /// Migrations directory
        #[arg(short, long, default_value = "migrations")]
        dir: PathBuf,
    },

    /// Create a new migration file
    Create {
        /// Migration name (e.g., "add status column")
        name: String,

        /// Migrations directory
        #[arg(short, long, default_value = "migrations")]
        dir: PathBuf,
    },
}

impl Cli {
    #[must_use] 
    pub fn parse_args() -> Self {
        Self::parse()
    }

    pub async fn execute(self) -> Result<()> {
        match self.command {
            | Some(Commands::Init {
                name,
                expiry_hours,
                output,
                yes,
            }) => {
                init_database(name, expiry_hours, output, yes).await?;
            },
            | Some(Commands::GenerateJwtSecret { output }) => {
                generate_jwt_secret(output)?;
            },
            | Some(Commands::Key { action }) => {
                handle_key_command(action).await?;
            },
            | Some(Commands::Migrate { action }) => {
                handle_migrate_command(action).await?;
            },
            | Some(Commands::HealthCheck { url, timeout }) => {
                health_check(url, timeout).await?;
            },
            | Some(Commands::Serve) | None => {
                // Will be handled by main.rs to start the server
                return Ok(());
            },
        }
        Ok(())
    }
}

async fn init_database(
    name: String,
    expiry_hours: u32,
    output: Option<PathBuf>,
    skip_confirm: bool,
) -> Result<()> {
    println!("🚀 NeuroQuantumDB Initialization");
    println!("================================\n");

    // Check if already initialized
    let state_file = PathBuf::from(".neuroquantum/initialized");
    if state_file.exists() {
        warn!("⚠️ Database appears to be already initialized!");
        if !skip_confirm {
            print!("Continue anyway? This will create a new admin key. [y/N]: ");
            io::stdout().flush()?;
            let mut response = String::new();
            io::stdin().read_line(&mut response)?;
            if !response.trim().eq_ignore_ascii_case("y") {
                println!("Initialization cancelled.");
                return Ok(());
            }
        }
    }

    println!("📋 Configuration:");
    println!("  Admin Key Name: {name}");
    println!(
        "  Expiry: {} hours ({} days)",
        expiry_hours,
        expiry_hours / 24
    );
    println!();

    if !skip_confirm {
        print!("Proceed with initialization? [Y/n]: ");
        io::stdout().flush()?;
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        if response.trim().eq_ignore_ascii_case("n") {
            println!("Initialization cancelled.");
            return Ok(());
        }
    }

    // Create auth service
    let mut auth_service = AuthService::new()
        .map_err(|e| anyhow::anyhow!("Failed to initialize auth service: {e}"))?;

    // Generate admin key
    println!("\n🔑 Generating admin API key...");
    let admin_key = auth_service
        .create_initial_admin_key(name.clone(), Some(expiry_hours))
        .map_err(|e| anyhow::anyhow!("Failed to create admin key: {e}"))?;

    // Display and save the key
    println!("\n✅ Admin API key created successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔐 API Key: {}", admin_key.key);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 Name: {}", admin_key.name);
    println!("⏰ Created: {}", admin_key.created_at);
    println!("⏳ Expires: {}", admin_key.expires_at);
    println!("🎫 Permissions: {}", admin_key.permissions.join(", "));
    println!();

    warn!("⚠️  IMPORTANT: Save this key securely - it will not be shown again!");
    warn!("⚠️  This key provides full admin access to your database.");

    // Save to file if requested
    if let Some(output_path) = output {
        let key_content = format!(
            "# NeuroQuantumDB Admin API Key\n\
             # Generated: {}\n\
             # Name: {}\n\
             # Expires: {}\n\
             # Permissions: {}\n\n\
             NEUROQUANTUM_API_KEY={}\n",
            admin_key.created_at,
            admin_key.name,
            admin_key.expires_at,
            admin_key.permissions.join(", "),
            admin_key.key
        );

        fs::write(&output_path, key_content).context("Failed to write API key to file")?;

        println!("💾 API key saved to: {}", output_path.display());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&output_path)?.permissions();
            perms.set_mode(0o600); // Read/write for owner only
            fs::set_permissions(&output_path, perms)?;
            println!("🔒 File permissions set to 600 (owner read/write only)");
        }
    }

    // Create state directory and mark as initialized
    fs::create_dir_all(".neuroquantum")?;
    fs::write(
        state_file,
        format!(
            "Initialized at: {}\nAdmin key: {}",
            chrono::Utc::now(),
            name
        ),
    )?;

    println!("\n✅ Initialization complete!");
    println!("💡 Next steps:");
    println!("   1. Export your API key: export NEUROQUANTUM_API_KEY=<your-key>");
    println!("   2. Start the server: neuroquantum-api serve");
    println!(
        "   3. Test the connection: curl -H \"Authorization: Bearer $NEUROQUANTUM_API_KEY\" http://localhost:8080/health"
    );

    Ok(())
}

fn generate_jwt_secret(output: Option<PathBuf>) -> Result<()> {
    use rand::Rng;

    println!("🔐 Generating secure JWT secret...\n");

    // Generate 64 bytes (512 bits) of random data
    let mut rng = rand::thread_rng();
    let secret_bytes: Vec<u8> = (0..64).map(|_| rng.gen()).collect();
    let secret = base64::engine::general_purpose::STANDARD.encode(&secret_bytes);

    println!("✅ JWT Secret generated successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("{secret}");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    warn!("⚠️  Add this to your config/prod.toml:");
    println!("   [auth]");
    println!("   jwt_secret = \"{secret}\"");

    if let Some(output_path) = output {
        let content = format!(
            "# NeuroQuantumDB JWT Secret\n\
             # Generated: {}\n\
             # Add this to your config/prod.toml under [auth] section\n\n\
             jwt_secret = \"{}\"\n",
            chrono::Utc::now(),
            secret
        );

        fs::write(&output_path, content).context("Failed to write JWT secret to file")?;
        println!("\n💾 JWT secret saved to: {}", output_path.display());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&output_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&output_path, perms)?;
            println!("🔒 File permissions set to 600 (owner read/write only)");
        }
    }

    Ok(())
}

async fn handle_key_command(action: KeyAction) -> Result<()> {
    match action {
        | KeyAction::Create {
            name,
            admin_key,
            permissions,
            expiry_hours,
            rate_limit,
            output,
        } => {
            create_api_key(
                name,
                admin_key,
                permissions,
                expiry_hours,
                rate_limit,
                output,
            )
            .await
        },
        | KeyAction::List { admin_key } => list_api_keys(admin_key).await,
        | KeyAction::Revoke { admin_key, key } => revoke_api_key(admin_key, key).await,
        | KeyAction::Stats { admin_key } => show_stats(admin_key).await,
    }
}

async fn create_api_key(
    name: String,
    admin_key: Option<String>,
    permissions: Vec<String>,
    expiry_hours: u32,
    rate_limit: Option<u32>,
    output: Option<PathBuf>,
) -> Result<()> {
    println!("🔑 Creating new API key...\n");

    // Get admin key from argument or environment variable
    let admin_key = admin_key
        .or_else(|| std::env::var("NEUROQUANTUM_ADMIN_KEY").ok())
        .ok_or_else(|| anyhow::anyhow!("Admin key required. Provide --admin-key or set NEUROQUANTUM_ADMIN_KEY environment variable"))?;

    let mut auth_service = AuthService::new()
        .map_err(|e| anyhow::anyhow!("Failed to initialize auth service: {e}"))?;

    let admin_api_key = auth_service
        .validate_api_key(&admin_key)
        .await
        .ok_or_else(|| anyhow::anyhow!("Invalid admin key"))?;

    if !admin_api_key.permissions.contains(&"admin".to_string()) {
        anyhow::bail!("Admin permission required to create API keys");
    }

    let valid_permissions = vec!["admin", "neuromorphic", "quantum", "dna", "read", "write"];
    for permission in &permissions {
        if !valid_permissions.contains(&permission.as_str()) {
            anyhow::bail!(
                "Invalid permission: {permission}. Valid permissions are: {valid_permissions:?}"
            );
        }
    }

    let new_key = auth_service
        .generate_api_key(
            name.clone(),
            permissions.clone(),
            Some(expiry_hours),
            rate_limit,
        )
        .map_err(|e| anyhow::anyhow!("Failed to generate API key: {e}"))?;

    println!("✅ API key created successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔐 API Key: {}", new_key.key);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 Name: {}", new_key.name);
    println!("⏰ Created: {}", new_key.created_at);
    println!("⏳ Expires: {}", new_key.expires_at);
    println!("🎫 Permissions: {}", new_key.permissions.join(", "));
    if let Some(limit) = new_key.rate_limit_per_hour {
        println!("⚡ Rate Limit: {limit} requests/hour");
    }
    println!();

    warn!("⚠️  IMPORTANT: Save this key securely - it will not be shown again!");

    if let Some(output_path) = output {
        let key_content = format!(
            "# NeuroQuantumDB API Key\n\
             # Generated: {}\n\
             # Name: {}\n\
             # Expires: {}\n\
             # Permissions: {}\n\n\
             NEUROQUANTUM_API_KEY={}\n",
            new_key.created_at,
            new_key.name,
            new_key.expires_at,
            new_key.permissions.join(", "),
            new_key.key
        );

        fs::write(&output_path, key_content).context("Failed to write API key to file")?;
        println!("💾 API key saved to: {}", output_path.display());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&output_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&output_path, perms)?;
            println!("🔒 File permissions set to 600 (owner read/write only)");
        }
    }

    Ok(())
}

async fn list_api_keys(admin_key: Option<String>) -> Result<()> {
    println!("📋 Listing all API keys...\n");

    // Get admin key from argument or environment variable
    let admin_key = admin_key
        .or_else(|| std::env::var("NEUROQUANTUM_ADMIN_KEY").ok())
        .ok_or_else(|| anyhow::anyhow!("Admin key required. Provide --admin-key or set NEUROQUANTUM_ADMIN_KEY environment variable"))?;

    let auth_service = AuthService::new()
        .map_err(|e| anyhow::anyhow!("Failed to initialize auth service: {e}"))?;

    let admin_api_key = auth_service
        .validate_api_key(&admin_key)
        .await
        .ok_or_else(|| anyhow::anyhow!("Invalid admin key"))?;

    if !admin_api_key.permissions.contains(&"admin".to_string()) {
        anyhow::bail!("Admin permission required to list API keys");
    }

    let keys = auth_service.list_api_keys();

    if keys.is_empty() {
        println!("No API keys found.");
        return Ok(());
    }

    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ API Keys                                                        │");
    println!("├─────────────────────────────────────────────────────────────────┤");

    for (i, key_info) in keys.iter().enumerate() {
        if i > 0 {
            println!("├─────────────────────────────────────────────────────────────────┤");
        }
        println!("│ Name: {:<58}│", key_info.name);
        println!("│ Key:  {:<58}│", key_info.key_id);
        println!("│ Permissions: {:<51}│", key_info.permissions.join(", "));
        println!(
            "│ Created: {:<55}│",
            key_info.created_at.format("%Y-%m-%d %H:%M:%S")
        );
        println!(
            "│ Expires: {:<55}│",
            key_info.expires_at.format("%Y-%m-%d %H:%M:%S")
        );
        println!("│ Usage: {} times{:<48}│", key_info.usage_count, "");
        if let Some(last_used) = key_info.last_used {
            println!(
                "│ Last Used: {:<53}│",
                last_used.format("%Y-%m-%d %H:%M:%S")
            );
        }
    }

    println!("└─────────────────────────────────────────────────────────────────┘");
    println!("\nTotal: {} API key(s)", keys.len());

    Ok(())
}

async fn revoke_api_key(admin_key: Option<String>, key_to_revoke: String) -> Result<()> {
    println!("🗑️  Revoking API key...\n");

    // Get admin key from argument or environment variable
    let admin_key = admin_key
        .or_else(|| std::env::var("NEUROQUANTUM_ADMIN_KEY").ok())
        .ok_or_else(|| anyhow::anyhow!("Admin key required. Provide --admin-key or set NEUROQUANTUM_ADMIN_KEY environment variable"))?;

    let mut auth_service = AuthService::new()
        .map_err(|e| anyhow::anyhow!("Failed to initialize auth service: {e}"))?;

    let admin_api_key = auth_service
        .validate_api_key(&admin_key)
        .await
        .ok_or_else(|| anyhow::anyhow!("Invalid admin key"))?;

    if !admin_api_key.permissions.contains(&"admin".to_string()) {
        anyhow::bail!("Admin permission required to revoke API keys");
    }

    let revoked = auth_service.revoke_api_key(&key_to_revoke, Some(&admin_api_key.name));

    if revoked {
        println!("✅ API key revoked successfully");
        println!(
            "🔑 Revoked key: {}...",
            &key_to_revoke[..16.min(key_to_revoke.len())]
        );
    } else {
        println!("❌ API key not found or already revoked");
    }

    Ok(())
}

async fn show_stats(admin_key: Option<String>) -> Result<()> {
    println!("📊 API Key Statistics\n");

    // Get admin key from argument or environment variable
    let admin_key = admin_key
        .or_else(|| std::env::var("NEUROQUANTUM_ADMIN_KEY").ok())
        .ok_or_else(|| anyhow::anyhow!("Admin key required. Provide --admin-key or set NEUROQUANTUM_ADMIN_KEY environment variable"))?;

    let auth_service = AuthService::new()
        .map_err(|e| anyhow::anyhow!("Failed to initialize auth service: {e}"))?;

    let admin_api_key = auth_service
        .validate_api_key(&admin_key)
        .await
        .ok_or_else(|| anyhow::anyhow!("Invalid admin key"))?;

    if !admin_api_key.permissions.contains(&"admin".to_string()) {
        anyhow::bail!("Admin permission required to view statistics");
    }

    let stats = auth_service.get_storage_stats();

    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ Storage Statistics                                              │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ Total Active Keys:    {:<42}│", stats.total_active_keys);
    println!("│ Total Revoked Keys:   {:<42}│", stats.total_revoked_keys);
    println!("│ Admin Keys:           {:<42}│", stats.admin_keys);
    println!("└─────────────────────────────────────────────────────────────────┘");

    Ok(())
}

/// Perform health check for Docker/Kubernetes
async fn health_check(url: String, timeout_secs: u64) -> Result<()> {
    use std::time::Duration;

    let health_url = format!("{}/health", url.trim_end_matches('/'));

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("Failed to create HTTP client")?;

    // Send health check request
    match client.get(&health_url).send().await {
        | Ok(response) => {
            if response.status().is_success() {
                // Exit with 0 for healthy
                std::process::exit(0);
            } else {
                eprintln!("Health check failed: HTTP {}", response.status());
                std::process::exit(1);
            }
        },
        | Err(e) => {
            eprintln!("Health check failed: {e}");
            std::process::exit(1);
        },
    }
}

async fn handle_migrate_command(action: MigrateAction) -> Result<()> {
    use neuroquantum_core::storage::{MigrationConfig, MigrationExecutor, MigrationExecutorConfig};

    match action {
        | MigrateAction::Up {
            dir,
            dry_run,
            verbose,
        } => {
            println!("🚀 Running migrations...\n");

            let config = MigrationExecutorConfig {
                config: MigrationConfig {
                    migrations_dir: dir,
                    dry_run,
                    ..Default::default()
                },
                verbose,
            };

            let executor = MigrationExecutor::new(config);
            executor.initialize().await?;

            let results = executor.migrate_up().await?;

            if results.is_empty() {
                println!("✅ No pending migrations");
            } else {
                println!("📊 Migration Results:\n");
                for result in results {
                    if result.success {
                        println!("  ✅ {} - {}ms", result.migration_id, result.duration_ms);
                    } else {
                        println!(
                            "  ❌ {} - Failed: {}",
                            result.migration_id,
                            result.error.unwrap_or_else(|| "Unknown error".to_string())
                        );
                    }
                }
                println!("\n✅ Migration complete");
            }
        },
        | MigrateAction::Down {
            count,
            dir,
            dry_run,
            verbose,
        } => {
            println!("⏪ Rolling back {count} migration(s)...\n");

            let config = MigrationExecutorConfig {
                config: MigrationConfig {
                    migrations_dir: dir,
                    dry_run,
                    ..Default::default()
                },
                verbose,
            };

            let executor = MigrationExecutor::new(config);
            executor.initialize().await?;

            let results = executor.migrate_down(count).await?;

            if results.is_empty() {
                println!("✅ No migrations to rollback");
            } else {
                println!("📊 Rollback Results:\n");
                for result in results {
                    if result.success {
                        println!("  ✅ {} - {}ms", result.migration_id, result.duration_ms);
                    } else {
                        println!(
                            "  ❌ {} - Failed: {}",
                            result.migration_id,
                            result.error.unwrap_or_else(|| "Unknown error".to_string())
                        );
                    }
                }
                println!("\n✅ Rollback complete");
            }
        },
        | MigrateAction::Status { dir } => {
            println!("📋 Migration Status\n");

            let config = MigrationExecutorConfig {
                config: MigrationConfig {
                    migrations_dir: dir,
                    ..Default::default()
                },
                verbose: false,
            };

            let executor = MigrationExecutor::new(config);
            executor.initialize().await?;

            let status = executor.status().await?;

            if status.is_empty() {
                println!("No migrations found");
            } else {
                println!("Migration ID          | Status   | Description");
                println!("----------------------|----------|--------------------");
                for (migration, applied) in status {
                    println!(
                        "{:<20}  | {:<8} | {}",
                        migration.id,
                        if applied {
                            "✅ Applied"
                        } else {
                            "⏳ Pending"
                        },
                        migration.description
                    );
                }
            }
        },
        | MigrateAction::Create { name, dir } => {
            println!("📝 Creating new migration: {name}\n");

            let config = MigrationExecutorConfig {
                config: MigrationConfig {
                    migrations_dir: dir,
                    ..Default::default()
                },
                verbose: false,
            };

            let executor = MigrationExecutor::new(config);

            let (up_file, down_file) = executor.create(&name)?;

            println!("✅ Created migration files:");
            println!("  Up:   {}", up_file.display());
            println!("  Down: {}", down_file.display());
            println!("\n💡 Edit these files to add your migration SQL");
        },
    }

    Ok(())
}
