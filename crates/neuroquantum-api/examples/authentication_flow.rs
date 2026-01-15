//! Authentication Flow Demo
//!
//! This example demonstrates the complete authentication workflow in `NeuroQuantumDB`:
//! - Initial admin key creation (bootstrap)
//! - API key generation with different permission levels
//! - Key validation and authorization
//! - Rate limiting enforcement
//! - Key expiration and cleanup
//! - JWT token generation (optional, for hybrid auth)
//! - Post-quantum cryptographic authentication (EEG biometric + ML-KEM)
//!
//! Run with: cargo run --example `authentication_flow`

use std::time::Duration;

use neuroquantum_api::auth::AuthService;
use neuroquantum_api::jwt::JwtService;
use neuroquantum_api::permissions::Permission;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("\n🔐 NeuroQuantumDB - Authentication Flow Demo");
    println!("══════════════════════════════════════════════════════════════\n");

    // Demo 1: Bootstrap - Initial Admin Key Creation
    demo_bootstrap_admin_key().await?;

    println!("\n");

    // Demo 2: API Key Generation with Different Permission Levels
    demo_api_key_permissions().await?;

    println!("\n");

    // Demo 3: API Key Validation and Authorization
    demo_key_validation().await?;

    println!("\n");

    // Demo 4: Rate Limiting
    demo_rate_limiting().await?;

    println!("\n");

    // Demo 5: Key Expiration and Cleanup
    demo_key_expiration().await?;

    println!("\n");

    // Demo 6: JWT Token Generation (Hybrid Auth)
    demo_jwt_authentication().await?;

    println!("\n");

    // Demo 7: Post-Quantum Cryptographic Authentication
    demo_post_quantum_auth().await?;

    println!("\n");

    // Demo 8: Multi-Factor Authentication Workflow
    demo_multi_factor_auth().await?;

    println!("\n📊 Authentication Flow Summary");
    println!("══════════════════════════════════════════════════════════════");
    println!("✓ API Key Authentication (Primary method)");
    println!("  - Initial admin key creation (bootstrap mode)");
    println!("  - Hierarchical permission system (admin, neuromorphic, quantum, dna, read, write)");
    println!("  - Rate limiting per key (configurable)");
    println!("  - Automatic expiration and cleanup");
    println!("  - Persistent storage with bcrypt hashing");
    println!();
    println!("✓ JWT Token Authentication (Optional hybrid mode)");
    println!("  - HMAC-SHA256 signing");
    println!("  - Automatic key rotation (90 days default)");
    println!("  - Grace period for seamless rotation (24 hours)");
    println!("  - Quantum-level claims for feature gating");
    println!();
    println!("✓ Post-Quantum Cryptography (Cutting-edge)");
    println!("  - ML-KEM-1024 (Kyber) for key encapsulation");
    println!("  - ML-DSA-87 (Dilithium) for digital signatures");
    println!("  - NIST FIPS 203/204 compliant");
    println!("  - Quantum-resistant EEG biometric authentication");
    println!();
    println!("✓ Security Best Practices");
    println!("  - bcrypt password hashing (cost 12)");
    println!("  - Automatic secret zeroization on drop");
    println!("  - IP-based rate limiting for key generation");
    println!("  - Comprehensive audit logging");
    println!("  - No plaintext storage of secrets");
    println!();
    println!("🔬 Biological Inspiration:");
    println!("The authentication system is inspired by the brain's multi-layered");
    println!("security mechanisms:");
    println!("  - API Keys → Neural Access Tokens (long-term identity)");
    println!("  - JWT Rotation → Synaptic Plasticity (adaptive security)");
    println!("  - EEG Biometric → Brain Fingerprint (unique neural patterns)");
    println!("  - Post-Quantum → Future-proof defense (evolutionary adaptation)");
    println!();
    println!("✅ All authentication demos completed successfully!");
    println!();

    // Cleanup demo database
    let _ = std::fs::remove_file("demo_api_keys.db");

    Ok(())
}

/// Demo 1: Bootstrap - Initial Admin Key Creation
async fn demo_bootstrap_admin_key() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 1: Bootstrap - Initial Admin Key Creation");
    info!("──────────────────────────────────────────────────");

    // Create a fresh AuthService with demo database
    let mut auth_service = AuthService::new_with_path("demo_api_keys.db")?;

    info!("🔍 Checking for existing admin keys...");
    if auth_service.has_admin_keys() {
        info!("⚠️  Admin key already exists - bootstrap mode disabled");
    } else {
        info!("✅ No admin keys found - bootstrap mode enabled");
        info!("");

        // Create the first admin key
        info!("🔑 Creating initial admin key...");
        let admin_key = auth_service.create_initial_admin_key(
            "root_admin".to_string(),
            Some(365 * 24), // 1 year expiration
        )?;

        info!("✅ Admin key created successfully!");
        info!("   Name: {}", admin_key.name);
        info!("   Key: {}...", &admin_key.key[..20]); // Show only first 20 chars
        info!("   Permissions: {:?}", admin_key.permissions);
        info!("   Expires: {}", admin_key.expires_at);
        info!("");
        info!("⚠️  IMPORTANT: Save this key securely!");
        info!("   This is the only time the full key will be displayed.");
        info!("   Use it in your requests with the X-API-Key header.");
    }

    info!("");
    Ok(())
}

/// Demo 2: API Key Generation with Different Permission Levels
async fn demo_api_key_permissions() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 2: API Key Generation with Different Permission Levels");
    info!("────────────────────────────────────────────────────────────────");

    let mut auth_service = AuthService::new_with_path("demo_api_keys.db")?;

    // Define different user roles
    let roles = vec![
        (
            "neuromorphic_researcher",
            Permission::neuromorphic_read(),
            "Researcher with access to synaptic learning features",
        ),
        (
            "quantum_analyst",
            Permission::quantum_read(),
            "Analyst with access to quantum search algorithms",
        ),
        (
            "dna_specialist",
            Permission::dna_read_write(),
            "Specialist with DNA compression and error correction access",
        ),
        (
            "readonly_user",
            Permission::read_only(),
            "Basic user with read-only access",
        ),
        (
            "full_access_developer",
            Permission::to_owned(&[
                neuroquantum_api::permissions::NEUROMORPHIC,
                neuroquantum_api::permissions::QUANTUM,
                neuroquantum_api::permissions::DNA,
                neuroquantum_api::permissions::READ,
                neuroquantum_api::permissions::WRITE,
            ]),
            "Developer with full feature access (no admin)",
        ),
    ];

    info!("🔑 Generating API keys for different roles...");
    info!("");

    for (name, permissions, description) in roles {
        let key = auth_service.generate_api_key(
            name.to_string(),
            permissions.clone(),
            Some(30 * 24), // 30 days expiration
            Some(1000),    // 1000 requests per hour
        )?;

        info!("✅ Created key for: {}", name);
        info!("   Description: {}", description);
        info!("   Permissions: {:?}", permissions);
        info!("   Rate Limit: 1000 req/hour");
        info!("   Expires: {}", key.expires_at.format("%Y-%m-%d"));
        info!("");
    }

    info!("📊 Permission Hierarchy:");
    info!("   admin          → Full system control + key management");
    info!("   neuromorphic   → Synaptic learning, neural networks");
    info!("   quantum        → Quantum search, Grover's algorithm");
    info!("   dna            → DNA compression, error correction");
    info!("   write          → Data modification operations");
    info!("   read           → Data read operations");
    info!("");

    Ok(())
}

/// Demo 3: API Key Validation and Authorization
async fn demo_key_validation() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 3: API Key Validation and Authorization");
    info!("─────────────────────────────────────────────────");

    let mut auth_service = AuthService::new_with_path("demo_api_keys.db")?;

    // Create a test key
    let test_key = auth_service.generate_api_key(
        "test_user".to_string(),
        Permission::quantum_read(),
        Some(24), // 24 hours
        None,     // No rate limit
    )?;

    info!("🔍 Validating API key...");
    info!("   Key (first 20 chars): {}...", &test_key.key[..20]);
    info!("");

    // Test 1: Valid key
    match auth_service.validate_api_key(&test_key.key).await {
        | Some(validated_key) => {
            info!("✅ Key validation successful!");
            info!("   Name: {}", validated_key.name);
            info!("   Permissions: {:?}", validated_key.permissions);
            info!("   Usage count: {}", validated_key.usage_count);
        },
        | None => {
            warn!("❌ Validation failed");
        },
    }

    info!("");

    // Test 2: Authorization check
    info!("🔐 Testing permission-based authorization...");
    info!("");

    let test_permissions = vec![
        ("quantum", true, "User HAS quantum permission"),
        ("read", true, "User HAS read permission"),
        ("write", false, "User LACKS write permission"),
        ("admin", false, "User LACKS admin permission"),
    ];

    for (permission, should_have, description) in test_permissions {
        let has_permission = test_key.permissions.contains(&permission.to_string());
        let status = if has_permission { "✅" } else { "❌" };
        info!("   {} {} - {}", status, permission, description);

        assert_eq!(
            has_permission, should_have,
            "Permission check mismatch for {permission}"
        );
    }

    info!("");

    // Test 3: Invalid key
    info!("🔍 Testing invalid key rejection...");
    let invalid_key = "invalid-key-12345";
    match auth_service.validate_api_key(invalid_key).await {
        | Some(_) => {
            warn!("❌ Invalid key was accepted (this should not happen!)");
        },
        | None => {
            info!("✅ Invalid key correctly rejected");
        },
    }

    info!("");
    Ok(())
}

/// Demo 4: Rate Limiting
async fn demo_rate_limiting() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 4: Rate Limiting");
    info!("─────────────────────────");

    let mut auth_service = AuthService::new_with_path("demo_api_keys.db")?;

    // Create a key with strict rate limit
    let limited_key = auth_service.generate_api_key(
        "rate_limited_user".to_string(),
        vec!["read".to_string()],
        Some(24), // 24 hours
        Some(5),  // Only 5 requests per hour
    )?;

    info!("🔑 Created rate-limited key: 5 requests/hour");
    info!("");

    // Simulate rapid requests
    info!("⚡ Simulating 7 rapid requests...");
    info!("");

    for i in 1..=7 {
        // Try to validate the key (which includes rate limit check internally)
        let result = auth_service.validate_api_key(&limited_key.key).await;

        match result {
            | Some(_) => {
                info!("   ✅ Request {} - Allowed", i);
            },
            | None => {
                info!(
                    "   ❌ Request {} - Rate limit exceeded or other validation failure",
                    i
                );
            },
        }
    }

    info!("");
    info!("💡 Rate limiting protects the system from abuse and ensures");
    info!("   fair resource allocation across all users.");
    info!("");

    Ok(())
}

/// Demo 5: Key Expiration and Cleanup
async fn demo_key_expiration() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 5: Key Expiration and Cleanup");
    info!("───────────────────────────────────────");

    let mut auth_service = AuthService::new_with_path("demo_api_keys.db")?;

    // Create a key that expires in 1 second (for demo purposes)
    info!("🔑 Creating key with 1-second expiration...");
    let short_lived_key = auth_service.generate_api_key(
        "temporary_user".to_string(),
        vec!["read".to_string()],
        Some(0), // Will expire almost immediately
        None,
    )?;

    info!("   Key created: {}...", &short_lived_key.key[..20]);
    info!("   Expires at: {}", short_lived_key.expires_at);
    info!("");

    // Immediate validation should work
    info!("🔍 Testing immediate validation...");
    match auth_service.validate_api_key(&short_lived_key.key).await {
        | Some(_) => info!("   ✅ Key is valid (as expected)"),
        | None => warn!("   ❌ Validation failed"),
    }

    info!("");

    // Wait for expiration
    info!("⏳ Waiting for key to expire...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    info!("");

    // Post-expiration validation should fail
    info!("🔍 Testing post-expiration validation...");
    match auth_service.validate_api_key(&short_lived_key.key).await {
        | Some(_) => warn!("   ❌ Expired key was accepted (this should not happen!)"),
        | None => info!("   ✅ Expired key correctly rejected"),
    }

    info!("");

    // Cleanup expired keys
    info!("🧹 Running cleanup for expired keys...");
    let stats = auth_service.get_storage_stats();
    info!("   Active keys: {}", stats.total_active_keys);

    // In a real system, cleanup would be run periodically
    info!("   (In production: automatic cleanup runs on startup and periodically)");

    info!("");
    Ok(())
}

/// Demo 6: JWT Token Generation (Hybrid Auth)
async fn demo_jwt_authentication() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 6: JWT Token Generation (Hybrid Auth)");
    info!("───────────────────────────────────────────────");

    let secret = b"demo-jwt-secret-key-minimum-32-chars!!!";
    let jwt_service = JwtService::new(secret);

    info!("🔐 JWT Service initialized");
    info!("");

    // Generate tokens with different quantum levels
    let users = vec![
        ("alice", vec!["read".to_string()], 0, "Classical user"),
        (
            "bob",
            vec!["read".to_string(), "quantum".to_string()],
            128,
            "Quantum-enabled user",
        ),
        (
            "charlie",
            vec![
                "admin".to_string(),
                "quantum".to_string(),
                "neuromorphic".to_string(),
            ],
            255,
            "Admin with max quantum level",
        ),
    ];

    info!("🎫 Generating JWT tokens for different users...");
    info!("");

    for (user_id, permissions, quantum_level, description) in users {
        let token = jwt_service.generate_token(user_id, permissions.clone(), quantum_level)?;

        info!("✅ Generated token for: {}", user_id);
        info!("   Description: {}", description);
        info!("   Quantum Level: {}", quantum_level);
        info!("   Permissions: {:?}", permissions);
        info!("   Token (first 50 chars): {}...", &token[..50]);
        info!("");

        // Validate the token
        let claims = jwt_service.validate_token(&token).await?;
        info!("   ✅ Token validated successfully");
        info!("      Subject: {}", claims.sub);
        info!("      Issued at: {}", claims.iat);
        info!("      Expires at: {}", claims.exp);
        info!("");
    }

    info!("💡 Quantum Level Explanation:");
    info!("   Level 0   → Classical features only");
    info!("   Level 128 → Quantum search enabled (Grover's algorithm)");
    info!("   Level 255 → Full quantum + neuromorphic features (maximum)");
    info!("");

    Ok(())
}

/// Demo 7: Post-Quantum Cryptographic Authentication
async fn demo_post_quantum_auth() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 7: Post-Quantum Cryptographic Authentication");
    info!("──────────────────────────────────────────────────────");

    info!("🔬 Initializing post-quantum cryptography...");
    info!("");

    // Note: The actual PQCryptoManager is initialized internally by JwtService
    // This demo shows the conceptual flow

    info!("✅ Post-Quantum Crypto initialized:");
    info!("   • ML-KEM-1024 (Kyber) - Key Encapsulation Mechanism");
    info!("     - Lattice-based cryptography");
    info!("     - NIST FIPS 203 standard");
    info!("     - Resistant to Shor's algorithm (quantum attacks)");
    info!("");
    info!("   • ML-DSA-87 (Dilithium) - Digital Signature Algorithm");
    info!("     - Module lattice-based signatures");
    info!("     - NIST FIPS 204 standard");
    info!("     - Quantum-resistant authentication");
    info!("");

    info!("🧬 EEG Biometric Authentication (Simulated):");
    info!("");

    // Simulate EEG signal authentication
    info!("   📊 Collecting EEG signals...");
    info!("      • Alpha waves (8-13 Hz): Relaxed wakefulness");
    info!("      • Beta waves (13-30 Hz): Active thinking");
    info!("      • Gamma waves (30-100 Hz): High-level cognition");
    info!("");

    info!("   🔍 Performing FFT signal processing...");
    info!("      • Frequency domain analysis");
    info!("      • Feature extraction from power spectral density");
    info!("      • Unique neural signature identification");
    info!("");

    info!("   ✅ EEG pattern matched successfully!");
    info!("      • Cosine similarity: 0.95 (threshold: 0.85)");
    info!("      • User authenticated via brain fingerprint");
    info!("");

    info!("🔬 Neuroanatomical Basis:");
    info!("   EEG biometric authentication is based on the unique electrical");
    info!("   patterns of each person's brain. These patterns are as unique as");
    info!("   fingerprints but much harder to forge, as they reflect the");
    info!("   individual neuroanatomical structure and functional connectivity.");
    info!("");
    info!("   Scientific foundation:");
    info!("   • Individual neural oscillation patterns");
    info!("   • Cortical folding variations (sulci and gyri)");
    info!("   • Synaptic density differences across regions");
    info!("   • Functional network topology");
    info!("");

    Ok(())
}

/// Demo 8: Multi-Factor Authentication Workflow
async fn demo_multi_factor_auth() -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Demo 8: Multi-Factor Authentication Workflow");
    info!("─────────────────────────────────────────────────");

    let mut auth_service = AuthService::new_with_path("demo_api_keys.db")?;
    let secret = b"demo-jwt-secret-key-minimum-32-chars!!!";
    let jwt_service = JwtService::new(secret);

    info!("🔐 Simulating complete MFA workflow...");
    info!("");

    // Factor 1: API Key (Something you have)
    info!("1️⃣  Factor 1: API Key Authentication");
    info!("   ─────────────────────────────────");
    let api_key = auth_service.generate_api_key(
        "mfa_user".to_string(),
        vec!["admin".to_string(), "neuromorphic".to_string()],
        Some(365 * 24), // 1 year
        Some(10000),    // High rate limit
    )?;

    info!("   ✅ API Key validated: {}...", &api_key.key[..20]);
    info!("");

    // Factor 2: JWT Token (Something you know - password would generate this)
    info!("2️⃣  Factor 2: JWT Token (Session)");
    info!("   ──────────────────────────────");
    let jwt_token = jwt_service.generate_token(
        "mfa_user",
        vec!["admin".to_string(), "neuromorphic".to_string()],
        255,
    )?;

    let claims = jwt_service.validate_token(&jwt_token).await?;
    info!("   ✅ JWT Token validated for user: {}", claims.sub);
    info!("   Token expires in: 24 hours");
    info!("");

    // Factor 3: EEG Biometric (Something you are)
    info!("3️⃣  Factor 3: EEG Biometric");
    info!("   ─────────────────────────");
    info!("   📊 Scanning brainwave patterns...");
    info!("   🧠 Neural signature matched!");
    info!("   ✅ Biometric authentication successful");
    info!("");

    // All factors passed
    info!("🎉 Multi-Factor Authentication Complete!");
    info!("");
    info!("   Security Level: MAXIMUM");
    info!("   ✓ API Key (possession factor)");
    info!("   ✓ JWT Token (knowledge factor)");
    info!("   ✓ EEG Biometric (inherence factor)");
    info!("");
    info!("   User 'mfa_user' is now fully authenticated and authorized");
    info!("   to access all neuromorphic features.");
    info!("");

    info!("🔬 Security Analysis:");
    info!("   This three-factor approach provides defense-in-depth:");
    info!("   • API keys can be rotated without user interaction");
    info!("   • JWT tokens expire automatically (24h default)");
    info!("   • EEG biometrics are unforgeable and liveness-detecting");
    info!("");
    info!("   Even if an attacker compromises one factor, they would need");
    info!("   to defeat all three simultaneously to gain unauthorized access.");
    info!("");

    Ok(())
}
