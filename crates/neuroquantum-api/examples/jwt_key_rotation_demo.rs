//! JWT Key Rotation Demo
//!
//! This example demonstrates the JWT secret key rotation feature.
//! Run with: cargo run --example jwt_key_rotation_demo

use neuroquantum_api::jwt::{JwtKeyRotation, JwtService};
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🔐 JWT Key Rotation Demo");
    info!("=======================\n");

    // Demo 1: Basic Key Rotation
    demo_basic_rotation().await?;

    // Demo 2: JWT Service with Rotation
    demo_jwt_service_rotation().await?;

    // Demo 3: Grace Period Validation
    demo_grace_period().await?;

    // Demo 4: Force Rotation (Emergency)
    demo_force_rotation().await?;

    info!("\n✅ All demos completed successfully!");

    Ok(())
}

/// Demo 1: Basic key rotation
async fn demo_basic_rotation() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 1: Basic Key Rotation");
    info!("------------------------------");

    let initial_secret = b"my-initial-secret-key-min-32-chars!!";
    let rotation_interval = Duration::from_secs(2); // 2 seconds for demo

    let rotation = JwtKeyRotation::new(initial_secret, rotation_interval);

    info!("Initial secret set");
    info!("Needs rotation: {}", rotation.needs_rotation().await);

    // Wait for rotation interval
    info!("Waiting 3 seconds for rotation interval...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    info!("Needs rotation: {}", rotation.needs_rotation().await);

    // Perform rotation
    if rotation.rotate().await? {
        info!("✅ Key rotation successful!");
    }

    // Verify previous key is available
    if rotation.previous_secret().await.is_some() {
        info!("✅ Previous key retained for grace period");
    }

    info!("");
    Ok(())
}

/// Demo 2: JWT Service with automatic rotation
async fn demo_jwt_service_rotation() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 2: JWT Service with Rotation");
    info!("-------------------------------------");

    let secret = b"jwt-service-secret-key-min-32-chars!";
    let rotation_interval = Duration::from_secs(90 * 24 * 3600); // 90 days

    let service = JwtService::with_rotation(secret, rotation_interval);

    // Generate a token
    let token = service.generate_token(
        "demo_user",
        vec!["read".to_string(), "write".to_string()],
        128,
    )?;

    info!("✅ Generated JWT token");
    info!("Token (truncated): {}...", &token[..50]);

    // Validate token
    let claims = service.validate_token(&token).await?;
    info!("✅ Token validated successfully");
    info!("User: {}", claims.sub);
    info!("Quantum Level: {}", claims.quantum_level);
    info!("Permissions: {:?}", claims.permissions);

    // Check rotation status
    if let Some(rotation_mgr) = service.rotation_manager() {
        let time_until = rotation_mgr.time_until_rotation().await?;
        info!(
            "⏰ Time until next rotation: {} days",
            time_until.as_secs() / (24 * 3600)
        );
    }

    info!("");
    Ok(())
}

/// Demo 3: Grace period validation
async fn demo_grace_period() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 3: Grace Period Validation");
    info!("-----------------------------------");

    let secret = b"grace-period-secret-key-min-32-chars";
    let rotation_interval = Duration::from_secs(2);
    let grace_period = Duration::from_secs(5);

    // Create service with rotation enabled
    let mut service = JwtService::with_rotation(secret, rotation_interval);

    // Generate token with initial key
    let token = service.generate_token("grace_user", vec!["test".to_string()], 64)?;

    info!("✅ Token generated with initial key");

    // Wait and rotate
    tokio::time::sleep(Duration::from_secs(3)).await;
    service.check_and_rotate().await?;
    info!("🔄 Keys rotated");

    // Token should still validate (grace period)
    match service.validate_token(&token).await {
        Ok(claims) => {
            info!("✅ Old token still valid during grace period");
            info!("User: {}", claims.sub);
        }
        Err(e) => {
            info!("❌ Token validation failed: {}", e);
        }
    }

    info!(
        "Grace period: {} seconds remaining",
        grace_period.as_secs() - 3
    );

    info!("");
    Ok(())
}

/// Demo 4: Force rotation (emergency scenario)
async fn demo_force_rotation() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 4: Force Rotation (Emergency)");
    info!("--------------------------------------");

    let secret = b"emergency-secret-key-min-32-chars!!!";
    let rotation_interval = Duration::from_secs(3600); // 1 hour

    let rotation = JwtKeyRotation::new(secret, rotation_interval);

    info!("Initial rotation interval: 1 hour");
    info!("Needs rotation: {}", rotation.needs_rotation().await);

    // Simulate key compromise - force immediate rotation
    info!("⚠️  Simulating key compromise...");
    info!("🚨 Forcing immediate rotation!");

    rotation.force_rotate().await?;

    info!("✅ Emergency rotation complete");
    info!("Previous key invalidated immediately (no grace period)");

    // Verify previous key is None
    if rotation.previous_secret().await.is_none() {
        info!("✅ Previous key cleared for security");
    }

    info!("");
    Ok(())
}
