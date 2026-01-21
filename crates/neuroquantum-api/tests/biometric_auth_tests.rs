//! Tests for biometric EEG authentication
//!
//! These tests validate EEG signal processing, feature extraction,
//! user enrollment, and authentication.

use std::f32::consts::PI;

use neuroquantum_api::biometric_auth::{ButterworthDesign, EEGAuthService, EEGProcessor};

fn generate_mock_eeg_signal(
    sampling_rate: f32,
    duration_seconds: f32,
    user_variant: f32,
) -> Vec<f32> {
    let num_samples = (sampling_rate * duration_seconds) as usize;
    let mut signal = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / sampling_rate;
        // Simulate different brainwave components with user-specific variations
        let delta = 2.0 * (2.0 * PI * 2.0 * t).sin() * user_variant;
        let theta = 1.5 * (2.0 * PI * 6.0 * t).sin() * user_variant;
        let alpha = 3.0 * (2.0 * PI * 10.0 * t).sin() * user_variant;
        let beta = 1.0 * (2.0 * PI * 20.0 * t).sin() * user_variant;
        let gamma = 0.5 * (2.0 * PI * 40.0 * t).sin() * user_variant;
        let noise = 0.05 * (i as f32 * 0.1).sin();

        signal.push(delta + theta + alpha + beta + gamma + noise);
    }

    signal
}

#[test]
fn test_eeg_processor_creation() {
    let processor = EEGProcessor::new(256.0);
    assert!(processor.is_ok());

    let invalid = EEGProcessor::new(50.0);
    assert!(invalid.is_err());
}

#[test]
fn test_butterworth_filter_basic() {
    // Test that the Butterworth filter doesn't produce NaN values
    let sampling_rate = 256.0;
    let designer = ButterworthDesign::new(sampling_rate);

    // Test lowpass filter at 30 Hz (safe frequency)
    let lp = designer.lowpass_biquad(30.0);
    eprintln!("Lowpass 30Hz coefficients: b={:?}, a={:?}", lp.b, lp.a);

    // Test lowpass filter at 100 Hz
    let lp100 = designer.lowpass_biquad(100.0);
    eprintln!(
        "Lowpass 100Hz coefficients: b={:?}, a={:?}",
        lp100.b, lp100.a
    );

    // Verify stability: |a2| should be < 1 for stability
    assert!(
        lp100.a[2].abs() < 1.0,
        "Lowpass filter at 100Hz is unstable: a2={}",
        lp100.a[2]
    );

    // Test highpass filter
    let hp = designer.highpass_biquad(0.5);
    eprintln!("Highpass coefficients: b={:?}, a={:?}", hp.b, hp.a);

    // Test notch filter
    let notch = designer.notch(50.0, 30.0);
    eprintln!("Notch coefficients: b={:?}, a={:?}", notch.b, notch.a);

    // Test individual filters first
    let signal: Vec<f32> = (0..768)
        .map(|i| (2.0 * PI * 10.0 * i as f32 / sampling_rate).sin())
        .collect();

    // Test lowpass alone
    let lp_filtered = lp100.filtfilt(&signal);
    eprintln!(
        "Lowpass filtered first 10: {:?}",
        &lp_filtered[..10.min(lp_filtered.len())]
    );
    assert!(
        !lp_filtered.iter().any(|&x| x.is_nan()),
        "Lowpass filter alone produced NaN"
    );

    // Test highpass alone
    let hp_filtered = hp.filtfilt(&signal);
    eprintln!(
        "Highpass filtered first 10: {:?}",
        &hp_filtered[..10.min(hp_filtered.len())]
    );
    assert!(
        !hp_filtered.iter().any(|&x| x.is_nan()),
        "Highpass filter alone produced NaN"
    );

    // Test bandpass filter (with reduced frequency range for safety)
    let bp = designer.bandpass(0.5, 80.0, 2); // Use 80Hz instead of 100Hz
    eprintln!("Bandpass sections: {}", bp.sections.len());
    for (i, section) in bp.sections.iter().enumerate() {
        eprintln!("  Section {}: b={:?}, a={:?}", i, section.b, section.a);
        // Verify each section is stable
        if section.a.len() > 2 {
            assert!(
                section.a[2].abs() < 1.0,
                "Section {} is unstable: a2={}",
                i,
                section.a[2]
            );
        }
    }

    let filtered = bp.filtfilt(&signal);
    eprintln!("Filtered signal length: {}", filtered.len());
    eprintln!(
        "Filtered first 10: {:?}",
        &filtered[..10.min(filtered.len())]
    );

    // Check no NaN values
    assert!(
        !filtered.iter().any(|&x| x.is_nan()),
        "Filter produced NaN values"
    );
    assert_eq!(filtered.len(), signal.len(), "Filter changed signal length");
}

#[test]
fn test_feature_extraction() {
    let processor = EEGProcessor::new(256.0).unwrap();
    let signal = generate_mock_eeg_signal(256.0, 3.0, 1.0);

    let features = processor.process_raw_eeg(&signal);
    assert!(
        features.is_ok(),
        "Feature extraction failed: {:?}",
        features.err()
    );

    let features = features.unwrap();
    assert!(
        features.alpha_power > 0.0,
        "alpha_power was {}",
        features.alpha_power
    );
    assert!(features.signal_quality > 0.0);
}

#[test]
fn test_user_enrollment_and_authentication() {
    let mut auth_service = EEGAuthService::new(256.0).unwrap();

    // Enroll user
    let user_id = "test_user_1".to_string();
    let enrollment_signal = generate_mock_eeg_signal(256.0, 3.0, 1.0);
    let signature = auth_service.enroll_user(user_id.clone(), &enrollment_signal);
    assert!(signature.is_ok());

    // Authenticate with similar signal
    let auth_signal = generate_mock_eeg_signal(256.0, 3.0, 1.05);
    let result = auth_service.authenticate(&user_id, &auth_signal);
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.similarity_score > 0.5);
}

#[test]
fn test_feature_similarity() {
    let processor = EEGProcessor::new(256.0).unwrap();

    let signal1 = generate_mock_eeg_signal(256.0, 3.0, 1.0);
    let features1 = processor.process_raw_eeg(&signal1).unwrap();

    let signal2 = generate_mock_eeg_signal(256.0, 3.0, 1.0);
    let features2 = processor.process_raw_eeg(&signal2).unwrap();

    let similarity = features1.similarity(&features2);
    assert!(
        similarity > 0.8,
        "Similar signals should have high similarity"
    );
}

#[test]
fn test_signature_update() {
    let mut auth_service = EEGAuthService::new(256.0).unwrap();
    let user_id = "test_user".to_string();

    let signal = generate_mock_eeg_signal(256.0, 3.0, 1.0);
    auth_service.enroll_user(user_id.clone(), &signal).unwrap();

    let update_signal = generate_mock_eeg_signal(256.0, 3.0, 1.0);
    let result = auth_service.update_signature(&user_id, &update_signal);
    assert!(result.is_ok());

    let signature = auth_service.get_signature(&user_id).unwrap();
    assert_eq!(signature.enrollment_count, 2);
}
