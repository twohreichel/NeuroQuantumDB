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
    /// Initialize NeuroQuantumDB with first admin key
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

    /// Start the API server (default command)
    Serve,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    pub async fn execute(self) -> Result<()> {
        match self.command {
            Some(Commands::Init {
                name,
                expiry_hours,
                output,
                yes,
            }) => {
                init_database(name, expiry_hours, output, yes).await?;
            }
            Some(Commands::GenerateJwtSecret { output }) => {
                generate_jwt_secret(output)?;
            }
            Some(Commands::Serve) | None => {
                // Will be handled by main.rs to start the server
                return Ok(());
            }
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
    println!("  Admin Key Name: {}", name);
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
    let mut auth_service = AuthService::new_with_setup_mode();

    // Generate admin key
    println!("\n🔑 Generating admin API key...");
    let admin_key = auth_service
        .create_initial_admin_key(name.clone(), Some(expiry_hours))
        .map_err(|e| anyhow::anyhow!("Failed to create admin key: {}", e))?;

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
    println!("   3. Test the connection: curl -H \"Authorization: Bearer $NEUROQUANTUM_API_KEY\" http://localhost:8080/health");

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
    println!("{}", secret);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    warn!("⚠️  Add this to your config/prod.toml:");
    println!("   [auth]");
    println!("   jwt_secret = \"{}\"", secret);

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
