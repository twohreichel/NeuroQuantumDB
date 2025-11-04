use anyhow::Result;
use neuroquantum_api::{cli::Cli, start_server, ApiConfig};
use std::env;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Set up panic handler first
    setup_panic_handler();

    // Parse CLI arguments
    let cli = Cli::parse_args();

    // Initialize logging (skip for CLI commands that don't need it)
    let needs_logging = matches!(cli.command, None | Some(neuroquantum_api::cli::Commands::Serve));
    if needs_logging {
        init_logging()?;
        print_banner();
        print_system_info();
    }

    // Handle CLI commands (init, generate-jwt-secret, etc.)
    if let Some(ref cmd) = cli.command {
        match cmd {
            neuroquantum_api::cli::Commands::Init { .. }
            | neuroquantum_api::cli::Commands::GenerateJwtSecret { .. } => {
                return cli.execute().await;
            }
            neuroquantum_api::cli::Commands::Serve => {
                // Continue to start server
            }
        }
    }

    // Load configuration
    let config = match ApiConfig::load() {
        Ok(config) => {
            info!("✅ Configuration loaded successfully");
            config
        }
        Err(e) => {
            error!("❌ Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Validate environment
    validate_environment(&config)?;

    // Set up graceful shutdown
    let shutdown_signal = setup_graceful_shutdown();

    // Start the server
    info!("🚀 Starting NeuroQuantumDB API Server...");
    info!("🌐 Server will be available at: {}", config.base_url());
    info!("📖 API Documentation: {}/api-docs/", config.base_url());
    info!("🏥 Health Check: {}/health", config.base_url());

    // Start server with graceful shutdown
    let server_future = start_server(config);

    tokio::select! {
        result = server_future => {
            match result {
                Ok(_) => info!("✅ Server stopped gracefully"),
                Err(e) => {
                    error!("❌ Server error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ = shutdown_signal => {
            info!("🛑 Received shutdown signal, stopping server...");
        }
    }

    info!("👋 NeuroQuantumDB API Server shutdown complete");
    Ok(())
}

/// Initialize logging based on environment
fn init_logging() -> Result<()> {
    let log_level = env::var("NEUROQUANTUM_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    let log_format = env::var("NEUROQUANTUM_LOG_FORMAT").unwrap_or_else(|_| "json".to_string());

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new(&log_level))?;

    match log_format.to_lowercase().as_str() {
        "json" => {
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_current_span(false)
                        .with_span_list(true),
                )
                .init();
        }
        "plain" | "pretty" => {
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .pretty()
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_line_number(true),
                )
                .init();
        }
        "compact" => {
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .compact()
                        .with_target(false),
                )
                .init();
        }
        _ => {
            // Default to JSON for unknown formats
            tracing_subscriber::registry()
                .with(filter)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
    }

    Ok(())
}

/// Print the startup banner
fn print_banner() {
    let banner = r#"
███╗   ██╗███████╗██╗   ██╗██████╗  ██████╗  ██████╗ ██╗   ██╗ █████╗ ███╗   ██╗████████╗██╗   ██╗███╗   ███╗
████╗  ██║██╔════╝██║   ██║██╔══██╗██╔═══██╗██╔═══██╗██║   ██║██╔══██╗████╗  ██║╚══██╔══╝██║   ██║████╗ ████║
██╔██╗ ██║█████╗  ██║   ██║██████╔╝██║   ██║██║   ██║██║   ██║███████║██╔██╗ ██║   ██║   ██║   ██║██╔████╔██║
██║╚██╗██║██╔══╝  ██║   ██║██╔══██╗██║   ██║██║▄▄ ██║██║   ██║██╔══██║██║╚██╗██║   ██║   ██║   ██║██║╚██╔╝██║
██║ ╚████║███████╗╚██████╔╝██║  ██║╚██████╔╝╚██████╔╝╚██████╔╝██║  ██║██║ ╚████║   ██║   ╚██████╔╝██║ ╚═╝ ██║
╚═╝  ╚═══╝╚══════╝ ╚═════╝ ╚═╝  ╚═╝ ╚═════╝  ╚══▀▀═╝  ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝    ╚═════╝ ╚═╝     ╚═╝

    ██████╗ ██████╗     █████╗ ██████╗ ██╗    ███████╗███████╗██████╗ ██╗   ██╗███████╗██████╗
    ██╔══██╗██╔══██╗   ██╔══██╗██╔══██╗██║    ██╔════╝██╔════╝██╔══██╗██║   ██║██╔════╝██╔══██╗
    ██║  ██║██████╔╝   ███████║██████╔╝██║    ███████╗█████╗  ██████╔╝██║   ██║█████╗  ██████╔╝
    ██║  ██║██╔══██╗   ██╔══██║██╔═══╝ ██║    ╚════██║██╔══╝  ██╔══██╗╚██╗ ██╔╝██╔══╝  ██╔══██╗
    ██████╔╝██████╔╝   ██║  ██║██║     ██║    ███████║███████╗██║  ██║ ╚████╔╝ ███████╗██║  ██║
    ╚═════╝ ╚═════╝    ╚═╝  ╚═╝╚═╝     ╚═╝    ╚══════╝╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚══════╝╚═╝  ╚═╝
    "#;

    info!("{}", banner);
    info!("🧠 Ultra-Efficient Neuromorphic Database for Edge Computing");
    info!("🚀 Version: {}", env!("CARGO_PKG_VERSION"));
    info!("⚛️  Features: Quantum Search | Neural Networks | DNA Compression");
    info!("🔒 Security: JWT Authentication | Rate Limiting | Quantum Encryption");
}

/// Validate the runtime environment
fn validate_environment(config: &ApiConfig) -> Result<()> {
    // Check if running as root when binding to privileged ports
    if config.server.port < 1024 {
        let user = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        if user != "root" {
            warn!(
                "⚠️  Attempting to bind to privileged port {} as non-root user '{}'",
                config.server.port, user
            );
            warn!("   This may fail. Consider using a port >= 1024 or running as root.");
        }
    }

    // Check available memory
    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        if let Some(line) = meminfo.lines().find(|l| l.starts_with("MemAvailable:")) {
            if let Some(mem_str) = line.split_whitespace().nth(1) {
                if let Ok(mem_kb) = mem_str.parse::<u64>() {
                    let mem_mb = mem_kb / 1024;
                    info!("💾 Available memory: {} MB", mem_mb);

                    if mem_mb < 512 {
                        warn!("⚠️  Low memory detected ({}MB). Consider increasing memory for optimal performance.", mem_mb);
                    }
                }
            }
        }
    }

    // Check disk space
    if let Ok(_metadata) = std::fs::metadata(".") {
        // This is a simplified check - in production you'd use statvfs or similar
        info!("💿 Current directory accessible");
    }

    // Validate network connectivity for Redis if configured
    if let Some(redis_config) = &config.redis {
        info!("🔗 Redis configured at: {}", redis_config.url);
        // Note: Actual connection testing happens in the RateLimitService initialization
    }

    // Check if JWT secret is production-ready
    if config.jwt.secret.contains("change-this") || config.jwt.secret.len() < 32 {
        warn!("🔐 JWT secret appears to be using default/weak value. Please set NEUROQUANTUM_JWT_SECRET environment variable.");
    }

    // Validate TLS configuration if enabled
    if let Some(tls_config) = &config.server.tls {
        if !std::path::Path::new(&tls_config.cert_file).exists() {
            error!(
                "❌ TLS certificate file not found: {}",
                tls_config.cert_file
            );
            return Err(anyhow::anyhow!("TLS certificate file missing"));
        }
        if !std::path::Path::new(&tls_config.key_file).exists() {
            error!("❌ TLS private key file not found: {}", tls_config.key_file);
            return Err(anyhow::anyhow!("TLS private key file missing"));
        }
        info!("🔒 TLS/HTTPS enabled");
    }

    Ok(())
}

/// Set up graceful shutdown signal handling
async fn setup_graceful_shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("📡 Received SIGINT (Ctrl+C)");
        },
        _ = terminate => {
            info!("📡 Received SIGTERM");
        },
    }
}

/// Print system information for debugging
fn print_system_info() {
    info!("🖥️  System Information:");
    info!("   OS: {}", env::consts::OS);
    info!("   Architecture: {}", env::consts::ARCH);
    let rust_version = env!("CARGO_PKG_RUST_VERSION");
    info!(
        "   Rust version: {}",
        if rust_version.is_empty() {
            "1.70+"
        } else {
            rust_version
        }
    );

    if let Ok(hostname) = env::var("HOSTNAME") {
        info!("   Hostname: {}", hostname);
    }

    if let Ok(user) = env::var("USER") {
        info!("   User: {}", user);
    }

    // CPU information
    let cpu_count = num_cpus::get();
    info!("   CPU cores: {}", cpu_count);

    // Environment variables of interest
    let env_vars = [
        "NEUROQUANTUM_CONFIG",
        "NEUROQUANTUM_HOST",
        "NEUROQUANTUM_PORT",
        "NEUROQUANTUM_LOG_LEVEL",
        "NEUROQUANTUM_REDIS_URL",
    ];

    for var in &env_vars {
        if let Ok(value) = env::var(var) {
            // Hide sensitive values
            let display_value = if var.contains("SECRET") || var.contains("PASSWORD") {
                "*".repeat(value.len().min(8))
            } else {
                value
            };
            info!("   {}: {}", var, display_value);
        }
    }
}

/// Handle panic with proper logging
fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .unwrap_or_else(|| std::panic::Location::caller());

        let msg = match panic_info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<dyn Any>",
            },
        };

        error!(
            "💥 PANIC occurred at {}:{}: {}",
            location.file(),
            location.line(),
            msg
        );

        // In production, you might want to send this to an error tracking service
        eprintln!(
            "💥 PANIC: {} at {}:{}",
            msg,
            location.file(),
            location.line()
        );

        std::process::exit(1);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_validation() {
        let config = ApiConfig::development();
        // This should not panic in test environment
        let result = std::panic::catch_unwind(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { validate_environment(&config) })
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_logging_initialization() {
        // Test that logging can be initialized without panicking
        std::env::set_var("NEUROQUANTUM_LOG_LEVEL", "debug");
        std::env::set_var("NEUROQUANTUM_LOG_FORMAT", "plain");

        let result = init_logging();
        assert!(result.is_ok());
    }
}
