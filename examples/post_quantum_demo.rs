/// Demonstration of Post-Quantum Cryptography Implementation
///
/// This example shows:
/// - ML-KEM (Kyber) key encapsulation
/// - ML-DSA (Dilithium) digital signatures
/// - Quantum-resistant token generation
/// - Integration with JWT authentication

use neuroquantum_core::pqcrypto::{PQCryptoManager, QuantumTokenClaims};
use anyhow::Result;

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║   Post-Quantum Cryptography Demo - NeuroQuantumDB     ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!();

    // Initialize post-quantum crypto manager
    println!("🔐 Initializing PQCryptoManager with ML-KEM-768 and ML-DSA-65...");
    let manager = PQCryptoManager::new();
    println!("✅ Initialized successfully");
    println!();

    // Display public keys
    println!("📋 Public Keys (Base64 encoded):");
    println!("   ML-KEM Public Key (first 64 chars): {}...",
        &manager.get_mlkem_public_key_base64()[..64]);
    println!("   ML-DSA Public Key (first 64 chars): {}...",
        &manager.get_mldsa_public_key_base64()[..64]);
    println!();

    // Demo 1: Digital Signatures
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 1: ML-DSA (Dilithium) Digital Signatures");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let message = b"Hello, Quantum-Resistant World!";
    println!("📝 Original message: {:?}", String::from_utf8_lossy(message));

    // Sign the message
    let start = std::time::Instant::now();
    let signature = manager.sign_message(message);
    let sign_time = start.elapsed();
    println!("✍️  Signed message (signature size: {} bytes) in {:?}",
        signature.len(), sign_time);

    // Verify the signature
    let start = std::time::Instant::now();
    match manager.verify_signature(&signature) {
        Ok(verified_msg) => {
            let verify_time = start.elapsed();
            println!("✅ Signature verified in {:?}", verify_time);
            println!("📝 Verified message: {:?}", String::from_utf8_lossy(&verified_msg));

            if verified_msg == message {
                println!("🎉 Message integrity confirmed!");
            }
        }
        Err(e) => {
            println!("❌ Signature verification failed: {}", e);
        }
    }
    println!();

    // Demo 2: Key Encapsulation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 2: ML-KEM (Kyber) Key Encapsulation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Encapsulate a shared secret
    let start = std::time::Instant::now();
    let (ciphertext, shared_secret1) = manager.encapsulate();
    let encap_time = start.elapsed();
    println!("🔒 Encapsulated shared secret in {:?}", encap_time);
    println!("   Ciphertext size: {} bytes", ciphertext.len());
    println!("   Shared secret size: {} bytes", shared_secret1.len());
    println!("   Shared secret (hex): {}...",
        hex::encode(&shared_secret1[..16]));

    // Decapsulate to recover the shared secret
    let start = std::time::Instant::now();
    match manager.decapsulate(&ciphertext) {
        Ok(shared_secret2) => {
            let decap_time = start.elapsed();
            println!("🔓 Decapsulated shared secret in {:?}", decap_time);

            if shared_secret1 == shared_secret2 {
                println!("✅ Shared secrets match!");
                println!("🎉 Secure key exchange completed!");
            } else {
                println!("❌ Shared secrets don't match!");
            }
        }
        Err(e) => {
            println!("❌ Decapsulation failed: {}", e);
        }
    }
    println!();

    // Demo 3: Quantum Token Generation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 3: Quantum-Resistant Authentication Tokens");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let user_id = "alice@neuroquantum.db";
    let session_id = "session_12345";

    println!("👤 User: {}", user_id);
    println!("🔑 Session: {}", session_id);
    println!();

    // Generate quantum token claims
    let start = std::time::Instant::now();
    match manager.generate_quantum_claims(user_id, session_id) {
        Ok(claims) => {
            let gen_time = start.elapsed();
            println!("✅ Generated quantum token claims in {:?}", gen_time);
            println!();
            println!("📋 Token Claims:");
            println!("   User ID: {}", claims.user_id);
            println!("   Session ID: {}", claims.session_id);
            println!("   Timestamp: {}", claims.timestamp);
            println!("   Quantum Signature (first 64 chars): {}...",
                &claims.quantum_signature[..64.min(claims.quantum_signature.len())]);
            println!("   KEM Ciphertext (first 64 chars): {}...",
                &claims.kem_ciphertext[..64.min(claims.kem_ciphertext.len())]);
            println!();

            // Verify the claims
            let start = std::time::Instant::now();
            match manager.verify_quantum_claims(&claims) {
                Ok(()) => {
                    let verify_time = start.elapsed();
                    println!("✅ Token claims verified in {:?}", verify_time);
                    println!("🎉 Authentication successful!");
                }
                Err(e) => {
                    println!("❌ Token verification failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Token generation failed: {}", e);
        }
    }
    println!();

    // Performance Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Performance Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("All operations completed with quantum-resistant security!");
    println!();
    println!("Security Level: NIST Level 3 (equivalent to AES-192)");
    println!("Algorithms:");
    println!("  - ML-KEM-768 for key encapsulation");
    println!("  - ML-DSA-65 for digital signatures");
    println!();
    println!("These algorithms are resistant to attacks from both");
    println!("classical and quantum computers, ensuring long-term security.");
    println!();

    Ok(())
}

// Helper function to convert bytes to hex string
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }
}

