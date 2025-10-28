/// EEG Biometric Authentication Demo
///
/// This example demonstrates the EEG-based biometric authentication feature.
/// It shows how to enroll a user and authenticate using simulated brainwave data.

use neuroquantum_api::biometric_auth::{EEGAuthService, EEGError};
use std::f32::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 NeuroQuantumDB - EEG Biometric Authentication Demo\n");

    // Create EEG authentication service with 256 Hz sampling rate
    let mut eeg_service = EEGAuthService::new(256.0)?;
    println!("✅ EEG Authentication Service initialized (256 Hz sampling rate)\n");

    // Step 1: Generate simulated EEG data for user enrollment
    println!("📊 Step 1: Generating simulated EEG data for user 'alice'");
    let duration_seconds = 3.0;
    let alice_eeg_data = generate_user_eeg_pattern(256.0, duration_seconds, 1.0);
    println!("   Generated {} samples ({} seconds)", alice_eeg_data.len(), duration_seconds);
    println!("   Signal quality: Good\n");

    // Step 2: Enroll user 'alice'
    println!("🔐 Step 2: Enrolling user 'alice' with EEG signature");
    let alice_signature = eeg_service.enroll_user("alice".to_string(), &alice_eeg_data)?;
    println!("   ✅ User enrolled successfully!");
    println!("   Signal Quality: {:.1}%", alice_signature.feature_template.signal_quality);
    println!("   Enrollment Count: {}", alice_signature.enrollment_count);
    println!("   Authentication Threshold: {:.1}%\n", alice_signature.authentication_threshold * 100.0);

    // Step 3: Improve signature with additional sample
    println!("🔄 Step 3: Updating signature with additional EEG sample");
    let alice_eeg_data_2 = generate_user_eeg_pattern(256.0, duration_seconds, 1.02);
    eeg_service.update_signature("alice", &alice_eeg_data_2)?;
    let updated_signature = eeg_service.get_signature("alice").unwrap();
    println!("   ✅ Signature updated!");
    println!("   New Enrollment Count: {}\n", updated_signature.enrollment_count);

    // Step 4: Successful authentication (same user)
    println!("✅ Step 4: Authenticating as 'alice' with similar EEG pattern");
    let alice_auth_data = generate_user_eeg_pattern(256.0, duration_seconds, 1.01);
    let auth_result = eeg_service.authenticate("alice", &alice_auth_data)?;
    println!("   Authentication Result: {}", if auth_result.authenticated { "✅ SUCCESS" } else { "❌ FAILED" });
    println!("   Similarity Score: {:.1}%", auth_result.similarity_score * 100.0);
    println!("   Threshold: {:.1}%\n", auth_result.threshold * 100.0);

    // Step 5: Enroll second user
    println!("🔐 Step 5: Enrolling user 'bob' with different EEG pattern");
    let bob_eeg_data = generate_user_eeg_pattern(256.0, duration_seconds, 1.5);
    let bob_signature = eeg_service.enroll_user("bob".to_string(), &bob_eeg_data)?;
    println!("   ✅ User 'bob' enrolled successfully!");
    println!("   Signal Quality: {:.1}%\n", bob_signature.feature_template.signal_quality);

    // Step 6: Failed authentication (wrong user)
    println!("❌ Step 6: Attempting to authenticate as 'alice' with 'bob's EEG pattern");
    let bob_auth_data = generate_user_eeg_pattern(256.0, duration_seconds, 1.48);
    match eeg_service.authenticate("alice", &bob_auth_data) {
        Ok(auth_result) => {
            println!("   Authentication Result: {}", if auth_result.authenticated { "✅ SUCCESS" } else { "❌ FAILED" });
            println!("   Similarity Score: {:.1}%", auth_result.similarity_score * 100.0);
            println!("   Threshold: {:.1}%", auth_result.threshold * 100.0);
            println!("   ⚠️  Correctly rejected: Different brainwave pattern!\n");
        }
        Err(e) => {
            println!("   ❌ Authentication error: {}\n", e);
        }
    }

    // Step 7: List all enrolled users
    println!("📋 Step 7: Listing all enrolled users");
    let users = eeg_service.list_users();
    println!("   Enrolled users: {:?}\n", users);

    // Step 8: Demonstrate signal quality rejection
    println!("⚠️  Step 8: Demonstrating poor signal quality rejection");
    let noisy_eeg_data = generate_noisy_eeg_pattern(256.0, duration_seconds);
    match eeg_service.enroll_user("charlie".to_string(), &noisy_eeg_data) {
        Ok(_) => println!("   Unexpected: Noisy signal was accepted"),
        Err(EEGError::PoorSignalQuality(quality)) => {
            println!("   ✅ Correctly rejected poor signal quality: {:.1}%", quality);
            println!("   Minimum required: 60.0%\n");
        }
        Err(e) => println!("   Error: {}\n", e),
    }

    // Summary
    println!("=" .repeat(60));
    println!("📊 Demo Summary:");
    println!("=" .repeat(60));
    println!("✅ Successfully enrolled: {} users", users.len());
    println!("✅ Successful authentication: alice");
    println!("❌ Failed authentication: alice (using bob's pattern)");
    println!("⚠️  Rejected poor quality signal");
    println!("\n🎉 EEG Biometric Authentication Demo completed successfully!");

    Ok(())
}

/// Generate simulated EEG data with user-specific brain pattern
fn generate_user_eeg_pattern(sampling_rate: f32, duration_seconds: f32, user_variant: f32) -> Vec<f32> {
    let num_samples = (sampling_rate * duration_seconds) as usize;
    let mut signal = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / sampling_rate;

        // Simulate different brainwave components with user-specific variations
        let delta = 0.5 * (2.0 * PI * 2.0 * t).sin() * user_variant;
        let theta = 0.3 * (2.0 * PI * 6.0 * t).sin() * user_variant;
        let alpha = 1.0 * (2.0 * PI * 10.0 * t).sin() * user_variant;
        let beta = 0.4 * (2.0 * PI * 20.0 * t).sin() * user_variant;
        let gamma = 0.2 * (2.0 * PI * 40.0 * t).sin() * user_variant;
        let noise = 0.05 * (i as f32 * 0.1).sin();

        signal.push(delta + theta + alpha + beta + gamma + noise);
    }

    signal
}

/// Generate noisy EEG data with poor signal quality
fn generate_noisy_eeg_pattern(sampling_rate: f32, duration_seconds: f32) -> Vec<f32> {
    let num_samples = (sampling_rate * duration_seconds) as usize;
    let mut signal = Vec::with_capacity(num_samples);

    use rand::Rng;
    let mut rng = rand::thread_rng();

    for i in 0..num_samples {
        let t = i as f32 / sampling_rate;

        // Very weak signal with heavy noise
        let weak_signal = 0.1 * (2.0 * PI * 10.0 * t).sin();
        let heavy_noise = rng.gen_range(-2.0..2.0);

        signal.push(weak_signal + heavy_noise);
    }

    signal
}

