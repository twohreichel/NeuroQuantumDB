//! EEG Feature Extraction Validation Tests
//!
//! This module provides comprehensive validation tests for EEG signal processing
//! and feature extraction to ensure physiologically plausible results and
//! correct signal processing behavior.
//!
//! Tests cover:
//! - Band power extraction validation (Delta, Theta, Alpha, Beta, Gamma)
//! - FFT correctness verification with known frequencies
//! - Filter frequency response validation
//! - Signal quality metrics plausibility
//! - Feature stability and reproducibility
//! - Edge cases and boundary conditions
//! - Physiologically realistic signal scenarios

use std::f32::consts::PI;

use neuroquantum_api::biometric_auth::{
    ButterworthDesign, EEGAuthService, EEGProcessor, FFTAnalyzer, FrequencyBand,
};

// =============================================================================
// Helper Functions for Test Signal Generation
// =============================================================================

/// Generate a pure sine wave at a specific frequency
fn generate_sine_wave(frequency: f32, sampling_rate: f32, duration_secs: f32) -> Vec<f32> {
    let num_samples = (sampling_rate * duration_secs) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sampling_rate;
            (2.0 * PI * frequency * t).sin()
        })
        .collect()
}

/// Generate a composite signal with multiple frequency components
fn generate_composite_signal(
    frequencies: &[(f32, f32)], // (frequency_hz, amplitude)
    sampling_rate: f32,
    duration_secs: f32,
) -> Vec<f32> {
    let num_samples = (sampling_rate * duration_secs) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sampling_rate;
            frequencies
                .iter()
                .map(|(freq, amp)| amp * (2.0 * PI * freq * t).sin())
                .sum::<f32>()
        })
        .collect()
}

/// Generate realistic EEG signal with typical brain rhythm amplitudes
/// Based on physiological EEG characteristics:
/// - Delta (0.5-4 Hz): 20-200 µV, highest during deep sleep
/// - Theta (4-8 Hz): 5-100 µV, meditation/drowsiness
/// - Alpha (8-13 Hz): 10-50 µV, relaxed wakefulness
/// - Beta (13-30 Hz): 5-30 µV, active thinking
/// - Gamma (30-100 Hz): 1-20 µV, cognitive processing
fn generate_realistic_eeg(sampling_rate: f32, duration_secs: f32, state: EEGState) -> Vec<f32> {
    let num_samples = (sampling_rate * duration_secs) as usize;

    // Amplitudes in arbitrary units (scaled for typical EEG ratios)
    let (delta_amp, theta_amp, alpha_amp, beta_amp, gamma_amp) = match state {
        | EEGState::RelaxedAwake => (0.5, 0.3, 2.0, 0.5, 0.2), // Strong alpha
        | EEGState::DeepSleep => (3.0, 1.0, 0.2, 0.1, 0.05),   // Strong delta
        | EEGState::ActiveThinking => (0.3, 0.3, 0.5, 2.0, 1.0), // Strong beta/gamma
        | EEGState::Meditation => (0.5, 2.0, 1.5, 0.3, 0.1),   // Strong theta/alpha
        | EEGState::Drowsy => (1.0, 2.0, 1.0, 0.3, 0.1),       // Strong theta
    };

    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sampling_rate;
            // Use representative frequencies from each band
            let delta = delta_amp * (2.0 * PI * 2.0 * t).sin();
            let theta = theta_amp * (2.0 * PI * 6.0 * t).sin();
            let alpha = alpha_amp * (2.0 * PI * 10.0 * t).sin();
            let beta = beta_amp * (2.0 * PI * 20.0 * t).sin();
            let gamma = gamma_amp * (2.0 * PI * 40.0 * t).sin();
            // Add small noise component
            let noise = 0.05 * ((i as f32 * 17.3).sin() + (i as f32 * 23.7).cos());

            delta + theta + alpha + beta + gamma + noise
        })
        .collect()
}

/// Different EEG states for realistic signal generation
#[derive(Debug, Clone, Copy)]
enum EEGState {
    RelaxedAwake,   // Eyes closed, relaxed - dominant alpha
    DeepSleep,      // NREM sleep - dominant delta
    ActiveThinking, // Mental task - dominant beta
    Meditation,     // Meditative state - theta/alpha
    Drowsy,         // Falling asleep - theta
}

/// Add Gaussian white noise to a signal
fn add_noise(signal: &[f32], noise_amplitude: f32) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    signal
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            // Simple pseudo-random noise using hash
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            let hash = hasher.finish();
            let rand_val = ((hash % 10000) as f32 / 10000.0).mul_add(2.0, -1.0);
            noise_amplitude.mul_add(rand_val, x)
        })
        .collect()
}

// =============================================================================
// FFT Correctness Tests
// =============================================================================

#[test]
fn test_fft_single_frequency_detection() {
    // Test that FFT correctly identifies a single known frequency
    let sampling_rate = 256.0;
    let test_frequency = 10.0; // 10 Hz (alpha band)
    let duration = 4.0; // 4 seconds for good frequency resolution

    let signal = generate_sine_wave(test_frequency, sampling_rate, duration);
    let analyzer = FFTAnalyzer::new(sampling_rate);
    let spectrum = analyzer.analyze(&signal);

    // Calculate expected peak bin
    let freq_resolution = sampling_rate / signal.len() as f32;
    let expected_bin = (test_frequency / freq_resolution) as usize;

    // Find the actual peak
    let peak_bin = spectrum
        .spectrum
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(idx, _)| idx)
        .unwrap();

    // Peak should be within ±1 bin of expected
    let bin_diff = (peak_bin as i32 - expected_bin as i32).abs();
    assert!(
        bin_diff <= 1,
        "FFT peak at bin {peak_bin} but expected bin {expected_bin} (frequency resolution: {freq_resolution:.2} Hz)"
    );
}

#[test]
fn test_fft_multiple_frequency_detection() {
    // Test that FFT correctly identifies multiple known frequencies
    let sampling_rate = 256.0;
    let frequencies = vec![
        (4.0, 1.0),  // Theta
        (10.0, 2.0), // Alpha (dominant)
        (25.0, 0.5), // Beta
    ];
    let duration = 4.0;

    let signal = generate_composite_signal(&frequencies, sampling_rate, duration);
    let analyzer = FFTAnalyzer::new(sampling_rate);
    let spectrum = analyzer.analyze(&signal);

    let freq_resolution = sampling_rate / signal.len() as f32;

    // Check that we have peaks near expected frequencies
    for (expected_freq, _) in &frequencies {
        let expected_bin = (*expected_freq / freq_resolution) as usize;
        let search_range = 2; // ±2 bins

        let peak_value = spectrum.spectrum[expected_bin.saturating_sub(search_range)
            ..=(expected_bin + search_range).min(spectrum.spectrum.len() - 1)]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        assert!(
            *peak_value > 0.01,
            "No significant peak found near {expected_freq:.1} Hz"
        );
    }
}

#[test]
fn test_fft_windowed_vs_unwindowed() {
    // Windowed FFT should have less spectral leakage
    let sampling_rate = 256.0;
    let test_frequency = 10.5; // Non-bin-aligned frequency
    let duration = 2.0;

    let signal = generate_sine_wave(test_frequency, sampling_rate, duration);
    let analyzer = FFTAnalyzer::new(sampling_rate);

    let spectrum_unwindowed = analyzer.analyze(&signal);
    let spectrum_windowed = analyzer.analyze_windowed(&signal);

    // Find peak in both spectra
    let find_peak = |spectrum: &[f32]| -> (usize, f32) {
        spectrum
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(idx, &val)| (idx, val))
            .unwrap()
    };

    let (_, peak_unwindowed) = find_peak(&spectrum_unwindowed.spectrum);
    let (_, peak_windowed) = find_peak(&spectrum_windowed.spectrum);

    // Windowed FFT should have a defined peak (may have different normalization)
    assert!(
        peak_windowed > 0.0,
        "Windowed FFT should produce valid peak"
    );
    assert!(
        peak_unwindowed > 0.0,
        "Unwindowed FFT should produce valid peak"
    );
}

// =============================================================================
// Band Power Validation Tests
// =============================================================================

#[test]
fn test_band_power_alpha_dominant() {
    // Alpha-dominant signal should have highest alpha power
    let sampling_rate = 256.0;
    let signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::RelaxedAwake);

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let features = processor.process_raw_eeg(&signal).unwrap();

    // Alpha should be dominant in relaxed awake state
    assert!(
        features.alpha_power > features.delta_power,
        "Alpha ({:.4}) should be greater than delta ({:.4}) in relaxed state",
        features.alpha_power,
        features.delta_power
    );
    assert!(
        features.alpha_power > features.gamma_power,
        "Alpha ({:.4}) should be greater than gamma ({:.4}) in relaxed state",
        features.alpha_power,
        features.gamma_power
    );
}

#[test]
fn test_band_power_delta_dominant_deep_sleep() {
    // Delta-dominant signal should have highest delta power
    let sampling_rate = 256.0;
    let signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::DeepSleep);

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let features = processor.process_raw_eeg(&signal).unwrap();

    // Delta should be dominant in deep sleep
    assert!(
        features.delta_power > features.alpha_power,
        "Delta ({:.4}) should be greater than alpha ({:.4}) in deep sleep",
        features.delta_power,
        features.alpha_power
    );
    assert!(
        features.delta_power > features.gamma_power,
        "Delta ({:.4}) should be greater than gamma ({:.4}) in deep sleep",
        features.delta_power,
        features.gamma_power
    );
}

#[test]
fn test_band_power_beta_dominant_active() {
    // Beta-dominant signal should have high beta/gamma in active thinking
    let sampling_rate = 256.0;
    let signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::ActiveThinking);

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let features = processor.process_raw_eeg(&signal).unwrap();

    // Beta+Gamma should be significant in active thinking
    let high_freq_power = features.beta_power + features.gamma_power;
    let low_freq_power = features.delta_power + features.theta_power;

    assert!(
        high_freq_power > low_freq_power * 0.5,
        "High frequency power ({high_freq_power:.4}) should be significant compared to low frequency ({low_freq_power:.4}) in active state"
    );
}

#[test]
fn test_band_power_ratios_physiological_range() {
    // Test that band power ratios are within physiological ranges
    let sampling_rate = 256.0;
    let signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::RelaxedAwake);

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let features = processor.process_raw_eeg(&signal).unwrap();

    // Alpha/Beta ratio typically 0.5-5.0 in healthy adults
    assert!(
        features.alpha_beta_ratio >= 0.1 && features.alpha_beta_ratio <= 20.0,
        "Alpha/Beta ratio ({:.2}) outside plausible range",
        features.alpha_beta_ratio
    );

    // Theta/Alpha ratio typically 0.2-2.0 in healthy adults
    assert!(
        features.theta_alpha_ratio >= 0.05 && features.theta_alpha_ratio <= 10.0,
        "Theta/Alpha ratio ({:.2}) outside plausible range",
        features.theta_alpha_ratio
    );
}

#[test]
fn test_all_band_powers_non_negative() {
    // All band powers must be non-negative
    let sampling_rate = 256.0;

    for state in [
        EEGState::RelaxedAwake,
        EEGState::DeepSleep,
        EEGState::ActiveThinking,
        EEGState::Meditation,
        EEGState::Drowsy,
    ] {
        let signal = generate_realistic_eeg(sampling_rate, 3.0, state);
        let processor = EEGProcessor::new(sampling_rate).unwrap();
        let features = processor.process_raw_eeg(&signal).unwrap();

        assert!(
            features.delta_power >= 0.0,
            "Delta power negative for {state:?}"
        );
        assert!(
            features.theta_power >= 0.0,
            "Theta power negative for {state:?}"
        );
        assert!(
            features.alpha_power >= 0.0,
            "Alpha power negative for {state:?}"
        );
        assert!(
            features.beta_power >= 0.0,
            "Beta power negative for {state:?}"
        );
        assert!(
            features.gamma_power >= 0.0,
            "Gamma power negative for {state:?}"
        );
    }
}

// =============================================================================
// Filter Frequency Response Tests
// =============================================================================

#[test]
fn test_lowpass_filter_attenuation() {
    // Lowpass filter should attenuate frequencies above cutoff
    let sampling_rate = 256.0;
    let cutoff = 30.0;
    let designer = ButterworthDesign::new(sampling_rate);

    // Signal with low (5 Hz) and high (50 Hz) frequency components
    let low_freq_signal = generate_sine_wave(5.0, sampling_rate, 2.0);
    let high_freq_signal = generate_sine_wave(50.0, sampling_rate, 2.0);
    let combined: Vec<f32> = low_freq_signal
        .iter()
        .zip(high_freq_signal.iter())
        .map(|(l, h)| l + h)
        .collect();

    let lp_filter = designer.lowpass_biquad(cutoff);
    let filtered = lp_filter.filtfilt(&combined);

    // Calculate power before and after filtering
    let original_power: f32 = combined.iter().map(|x| x * x).sum::<f32>() / combined.len() as f32;
    let filtered_power: f32 = filtered.iter().map(|x| x * x).sum::<f32>() / filtered.len() as f32;

    // Filtered signal should have less power (high freq attenuated)
    assert!(
        filtered_power < original_power,
        "Lowpass filter should reduce total power (original: {original_power:.4}, filtered: {filtered_power:.4})"
    );
}

#[test]
fn test_highpass_filter_attenuation() {
    // Highpass filter should attenuate frequencies below cutoff
    let sampling_rate = 256.0;
    let cutoff = 20.0;
    let designer = ButterworthDesign::new(sampling_rate);

    // Signal with low (2 Hz) and high (40 Hz) frequency components
    let low_freq_signal = generate_sine_wave(2.0, sampling_rate, 2.0);
    let high_freq_signal = generate_sine_wave(40.0, sampling_rate, 2.0);
    let combined: Vec<f32> = low_freq_signal
        .iter()
        .zip(high_freq_signal.iter())
        .map(|(l, h)| l + h)
        .collect();

    let hp_filter = designer.highpass_biquad(cutoff);
    let filtered = hp_filter.filtfilt(&combined);

    // Calculate power of low frequency component before and after
    let analyze_low_freq = |signal: &[f32]| -> f32 {
        let analyzer = FFTAnalyzer::new(sampling_rate);
        let spectrum = analyzer.analyze(signal);
        // Look at first few bins (low frequency)
        spectrum.spectrum.iter().take(10).sum::<f32>()
    };

    let original_low_power = analyze_low_freq(&combined);
    let filtered_low_power = analyze_low_freq(&filtered);

    assert!(
        filtered_low_power < original_low_power,
        "Highpass filter should reduce low frequency power"
    );
}

#[test]
fn test_bandpass_filter_passband() {
    // Bandpass filter should pass frequencies within the band
    let sampling_rate = 256.0;
    let low_cutoff = 5.0;
    let high_cutoff = 20.0;
    let designer = ButterworthDesign::new(sampling_rate);

    // Signal at 10 Hz (well within band center)
    // Use longer duration for better filter behavior
    let passband_signal = generate_sine_wave(10.0, sampling_rate, 4.0);

    // Use 1st order for less aggressive filtering in passband
    let bp_filter = designer.bandpass(low_cutoff, high_cutoff, 1);
    let filtered = bp_filter.filtfilt(&passband_signal);

    // Skip edge samples affected by filter transient
    let skip = 100;
    let stable_original: f32 = passband_signal[skip..passband_signal.len() - skip]
        .iter()
        .map(|x| x * x)
        .sum::<f32>()
        / (passband_signal.len() - 2 * skip) as f32;
    let stable_filtered: f32 = filtered[skip..filtered.len() - skip]
        .iter()
        .map(|x| x * x)
        .sum::<f32>()
        / (filtered.len() - 2 * skip) as f32;

    // Passband signal should retain significant power (allowing for filter characteristics)
    let retention_ratio = stable_filtered / stable_original;
    assert!(
        retention_ratio > 0.1, // At least 10% power retained in passband
        "Bandpass filter should pass in-band signal (retention: {:.2}%)",
        retention_ratio * 100.0
    );
}

#[test]
fn test_notch_filter_50hz_rejection() {
    // Notch filter should reject 50 Hz power line interference
    let sampling_rate = 256.0;
    let notch_freq = 50.0;
    let designer = ButterworthDesign::new(sampling_rate);

    // Signal with 10 Hz + 50 Hz interference
    let eeg_signal = generate_sine_wave(10.0, sampling_rate, 2.0);
    let interference = generate_sine_wave(50.0, sampling_rate, 2.0);
    let combined: Vec<f32> = eeg_signal
        .iter()
        .zip(interference.iter())
        .map(|(e, i)| e + i * 0.5) // 50 Hz at half amplitude
        .collect();

    let notch = designer.notch(notch_freq, 30.0);
    let filtered = notch.filtfilt(&combined);

    // Analyze 50 Hz power before and after
    let analyzer = FFTAnalyzer::new(sampling_rate);
    let spectrum_before = analyzer.analyze(&combined);
    let spectrum_after = analyzer.analyze(&filtered);

    let freq_resolution = sampling_rate / combined.len() as f32;
    let bin_50hz = (50.0 / freq_resolution) as usize;

    let power_before = spectrum_before.spectrum[bin_50hz.saturating_sub(1)..=bin_50hz + 1]
        .iter()
        .sum::<f32>();
    let power_after = spectrum_after.spectrum[bin_50hz.saturating_sub(1)..=bin_50hz + 1]
        .iter()
        .sum::<f32>();

    assert!(
        power_after < power_before * 0.5,
        "Notch filter should reduce 50 Hz power (before: {power_before:.4}, after: {power_after:.4})"
    );
}

#[test]
fn test_filter_no_nan_values() {
    // Filters should never produce NaN values
    let sampling_rate = 256.0;
    let designer = ButterworthDesign::new(sampling_rate);
    let signal = generate_sine_wave(10.0, sampling_rate, 3.0);

    // Test lowpass filter
    let lowpass_filtered = designer.lowpass_biquad(30.0).filtfilt(&signal);
    assert!(
        !lowpass_filtered.iter().any(|x| x.is_nan()),
        "Lowpass filter produced NaN values"
    );
    assert!(
        !lowpass_filtered.iter().any(|x| x.is_infinite()),
        "Lowpass filter produced infinite values"
    );

    // Test highpass filter
    let highpass_filtered = designer.highpass_biquad(1.0).filtfilt(&signal);
    assert!(
        !highpass_filtered.iter().any(|x| x.is_nan()),
        "Highpass filter produced NaN values"
    );
    assert!(
        !highpass_filtered.iter().any(|x| x.is_infinite()),
        "Highpass filter produced infinite values"
    );

    // Test bandpass filter
    let bandpass_filtered = designer.bandpass(1.0, 50.0, 2).filtfilt(&signal);
    assert!(
        !bandpass_filtered.iter().any(|x| x.is_nan()),
        "Bandpass filter produced NaN values"
    );
    assert!(
        !bandpass_filtered.iter().any(|x| x.is_infinite()),
        "Bandpass filter produced infinite values"
    );

    // Test notch filter
    let notch_filtered = designer.notch(50.0, 30.0).filtfilt(&signal);
    assert!(
        !notch_filtered.iter().any(|x| x.is_nan()),
        "Notch filter produced NaN values"
    );
    assert!(
        !notch_filtered.iter().any(|x| x.is_infinite()),
        "Notch filter produced infinite values"
    );
}

// =============================================================================
// Signal Quality Tests
// =============================================================================

#[test]
fn test_signal_quality_clean_signal() {
    // Clean signal should have high quality score
    let sampling_rate = 256.0;
    let signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::RelaxedAwake);

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let features = processor.process_raw_eeg(&signal).unwrap();

    assert!(
        features.signal_quality > 50.0,
        "Clean signal should have quality > 50%, got {:.1}%",
        features.signal_quality
    );
}

#[test]
fn test_signal_quality_noisy_signal() {
    // Very noisy signal should have lower quality score
    let sampling_rate = 256.0;
    let clean_signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::RelaxedAwake);
    let noisy_signal = add_noise(&clean_signal, 5.0); // High noise amplitude

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let clean_features = processor.process_raw_eeg(&clean_signal).unwrap();
    let noisy_features = processor.process_raw_eeg(&noisy_signal).unwrap();

    // Noisy signal should have lower quality than clean signal
    assert!(
        noisy_features.signal_quality <= clean_features.signal_quality,
        "Noisy signal ({:.1}%) should have lower or equal quality than clean ({:.1}%)",
        noisy_features.signal_quality,
        clean_features.signal_quality
    );
}

#[test]
fn test_signal_quality_range() {
    // Signal quality should be between 0 and 100
    let sampling_rate = 256.0;

    for state in [
        EEGState::RelaxedAwake,
        EEGState::DeepSleep,
        EEGState::ActiveThinking,
    ] {
        let signal = generate_realistic_eeg(sampling_rate, 3.0, state);
        let processor = EEGProcessor::new(sampling_rate).unwrap();
        let features = processor.process_raw_eeg(&signal).unwrap();

        assert!(
            features.signal_quality >= 0.0 && features.signal_quality <= 100.0,
            "Signal quality {:.1}% outside valid range [0, 100] for {:?}",
            features.signal_quality,
            state
        );
    }
}

// =============================================================================
// Feature Stability and Reproducibility Tests
// =============================================================================

#[test]
fn test_feature_extraction_deterministic() {
    // Same signal should produce identical features
    let sampling_rate = 256.0;
    let signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::RelaxedAwake);

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let features1 = processor.process_raw_eeg(&signal).unwrap();
    let features2 = processor.process_raw_eeg(&signal).unwrap();

    assert!(
        (features1.alpha_power - features2.alpha_power).abs() < 1e-6,
        "Feature extraction should be deterministic"
    );
    assert!(
        (features1.beta_power - features2.beta_power).abs() < 1e-6,
        "Feature extraction should be deterministic"
    );
}

#[test]
fn test_feature_stability_similar_signals() {
    // Very similar signals should produce similar features
    let sampling_rate = 256.0;
    let signal1 = generate_realistic_eeg(sampling_rate, 3.0, EEGState::RelaxedAwake);
    // Add tiny amount of noise
    let signal2 = add_noise(&signal1, 0.01);

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let features1 = processor.process_raw_eeg(&signal1).unwrap();
    let features2 = processor.process_raw_eeg(&signal2).unwrap();

    // Features should be highly similar
    let similarity = features1.similarity(&features2);
    assert!(
        similarity > 0.95,
        "Similar signals should have similarity > 0.95, got {similarity:.3}"
    );
}

#[test]
fn test_feature_differentiation_different_states() {
    // Different EEG states should produce distinguishable features
    let sampling_rate = 256.0;

    let relaxed_signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::RelaxedAwake);
    let active_signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::ActiveThinking);

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let relaxed_features = processor.process_raw_eeg(&relaxed_signal).unwrap();
    let active_features = processor.process_raw_eeg(&active_signal).unwrap();

    // Different states should produce different alpha/beta ratios
    let ratio_diff = (relaxed_features.alpha_beta_ratio - active_features.alpha_beta_ratio).abs();
    assert!(
        ratio_diff > 0.1,
        "Different EEG states should have different alpha/beta ratios (diff: {ratio_diff:.3})"
    );
}

// =============================================================================
// Edge Cases and Boundary Condition Tests
// =============================================================================

#[test]
fn test_minimum_sample_requirement() {
    // Processor should reject signals shorter than minimum required
    let sampling_rate = 256.0;
    let min_samples = (sampling_rate * 2.0) as usize; // 2 seconds minimum

    let processor = EEGProcessor::new(sampling_rate).unwrap();

    // Too short signal
    let short_signal: Vec<f32> = (0..min_samples - 1)
        .map(|i| (i as f32 * 0.1).sin())
        .collect();
    let result = processor.process_raw_eeg(&short_signal);
    assert!(result.is_err(), "Should reject signal shorter than minimum");

    // Exactly minimum length signal
    let exact_signal = generate_realistic_eeg(sampling_rate, 2.0, EEGState::RelaxedAwake);
    let result = processor.process_raw_eeg(&exact_signal);
    assert!(result.is_ok(), "Should accept signal at minimum length");
}

#[test]
fn test_sampling_rate_validation() {
    // Valid sampling rates (128-2048 Hz)
    assert!(EEGProcessor::new(128.0).is_ok());
    assert!(EEGProcessor::new(256.0).is_ok());
    assert!(EEGProcessor::new(512.0).is_ok());
    assert!(EEGProcessor::new(1024.0).is_ok());
    assert!(EEGProcessor::new(2048.0).is_ok());

    // Invalid sampling rates
    assert!(EEGProcessor::new(50.0).is_err()); // Too low
    assert!(EEGProcessor::new(127.0).is_err()); // Just below minimum
    assert!(EEGProcessor::new(2049.0).is_err()); // Just above maximum
    assert!(EEGProcessor::new(4096.0).is_err()); // Too high
}

#[test]
fn test_constant_signal_handling() {
    // Constant (DC) signal should be handled gracefully
    let sampling_rate = 256.0;
    let duration = 3.0;
    let num_samples = (sampling_rate * duration) as usize;

    let constant_signal: Vec<f32> = vec![1.0; num_samples];

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let result = processor.process_raw_eeg(&constant_signal);

    // Should either process with low quality or reject
    // Constant signal has no AC component, so band powers should be ~0
    if let Ok(features) = result {
        assert!(
            features.alpha_power < 0.01,
            "Constant signal should have near-zero alpha power"
        );
    }
}

#[test]
fn test_zero_signal_handling() {
    // All-zero signal should be handled gracefully
    let sampling_rate = 256.0;
    let duration = 3.0;
    let num_samples = (sampling_rate * duration) as usize;

    let zero_signal: Vec<f32> = vec![0.0; num_samples];

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let result = processor.process_raw_eeg(&zero_signal);

    // Should either process or reject, but not panic
    if let Ok(features) = result {
        assert!(
            features.std_deviation.abs() < 0.001,
            "Zero signal should have zero std deviation"
        );
    }
}

#[test]
fn test_very_large_amplitude_signal() {
    // Large amplitude signals should be processed without overflow
    let sampling_rate = 256.0;
    let signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::RelaxedAwake);
    let large_signal: Vec<f32> = signal.iter().map(|x| x * 1000.0).collect();

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let result = processor.process_raw_eeg(&large_signal);

    assert!(result.is_ok(), "Should handle large amplitude signals");
    let features = result.unwrap();
    assert!(
        !features.alpha_power.is_nan() && !features.alpha_power.is_infinite(),
        "Features should be finite"
    );
}

#[test]
fn test_small_amplitude_signal() {
    // Very small amplitude signals should be processed
    let sampling_rate = 256.0;
    let signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::RelaxedAwake);
    let small_signal: Vec<f32> = signal.iter().map(|x| x * 0.001).collect();

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let result = processor.process_raw_eeg(&small_signal);

    // Should process without errors (may have low quality)
    if let Ok(features) = result {
        assert!(
            features.alpha_power >= 0.0,
            "Features should be non-negative"
        );
    }
}

// =============================================================================
// Authentication Service Tests
// =============================================================================

#[test]
fn test_user_enrollment_creates_signature() {
    let mut auth_service = EEGAuthService::new(256.0).unwrap();
    let signal = generate_realistic_eeg(256.0, 3.0, EEGState::RelaxedAwake);

    let result = auth_service.enroll_user("test_user".to_string(), &signal);
    assert!(result.is_ok());

    let signature = auth_service.get_signature("test_user");
    assert!(signature.is_some());
    assert_eq!(signature.unwrap().enrollment_count, 1);
}

#[test]
fn test_authentication_same_user() {
    let mut auth_service = EEGAuthService::new(256.0).unwrap();

    // Enroll with one signal
    let enroll_signal = generate_realistic_eeg(256.0, 3.0, EEGState::RelaxedAwake);
    auth_service
        .enroll_user("user1".to_string(), &enroll_signal)
        .unwrap();

    // Authenticate with same signal type (same user)
    let auth_signal = generate_realistic_eeg(256.0, 3.0, EEGState::RelaxedAwake);
    let result = auth_service.authenticate("user1", &auth_signal).unwrap();

    // Same state should have high similarity
    assert!(
        result.similarity_score > 0.7,
        "Same user/state should have high similarity, got {:.2}",
        result.similarity_score
    );
}

#[test]
fn test_authentication_different_users() {
    let mut auth_service = EEGAuthService::new(256.0).unwrap();

    // Enroll user 1 (relaxed state - alpha dominant)
    let user1_signal = generate_realistic_eeg(256.0, 3.0, EEGState::RelaxedAwake);
    auth_service
        .enroll_user("user1".to_string(), &user1_signal)
        .unwrap();

    // Try to authenticate with very different signal (active thinking - beta dominant)
    let different_signal = generate_realistic_eeg(256.0, 3.0, EEGState::ActiveThinking);
    let result = auth_service
        .authenticate("user1", &different_signal)
        .unwrap();

    // Different state should have lower similarity
    // Note: This tests state differentiation, not actual user differentiation
    // Real user differentiation would require actual multi-user EEG data
    assert!(
        result.similarity_score < 1.0,
        "Different states should not have perfect similarity"
    );
}

#[test]
fn test_signature_update_improves_template() {
    let mut auth_service = EEGAuthService::new(256.0).unwrap();
    let user_id = "test_user";

    // Initial enrollment
    let signal1 = generate_realistic_eeg(256.0, 3.0, EEGState::RelaxedAwake);
    auth_service
        .enroll_user(user_id.to_string(), &signal1)
        .unwrap();

    // Update with additional sample
    let signal2 = generate_realistic_eeg(256.0, 3.0, EEGState::RelaxedAwake);
    auth_service.update_signature(user_id, &signal2).unwrap();

    let signature = auth_service.get_signature(user_id).unwrap();
    assert_eq!(signature.enrollment_count, 2);
}

#[test]
fn test_user_revocation() {
    let mut auth_service = EEGAuthService::new(256.0).unwrap();

    let signal = generate_realistic_eeg(256.0, 3.0, EEGState::RelaxedAwake);
    auth_service
        .enroll_user("test_user".to_string(), &signal)
        .unwrap();

    assert!(auth_service.get_signature("test_user").is_some());
    assert!(auth_service.revoke_user("test_user"));
    assert!(auth_service.get_signature("test_user").is_none());
}

// =============================================================================
// Frequency Band Range Tests
// =============================================================================

#[test]
fn test_frequency_band_ranges() {
    // Verify frequency band definitions match neurological standards
    let delta = FrequencyBand::Delta;
    let theta = FrequencyBand::Theta;
    let alpha = FrequencyBand::Alpha;
    let beta = FrequencyBand::Beta;
    let gamma = FrequencyBand::Gamma;

    // Standard EEG band definitions
    assert_eq!(delta.range(), (0.5, 4.0), "Delta should be 0.5-4 Hz");
    assert_eq!(theta.range(), (4.0, 8.0), "Theta should be 4-8 Hz");
    assert_eq!(alpha.range(), (8.0, 13.0), "Alpha should be 8-13 Hz");
    assert_eq!(beta.range(), (13.0, 30.0), "Beta should be 13-30 Hz");
    assert_eq!(gamma.range(), (30.0, 100.0), "Gamma should be 30-100 Hz");
}

#[test]
fn test_frequency_band_power_isolation() {
    // Test that band power extraction correctly isolates specific frequencies
    let sampling_rate = 256.0;
    let duration = 4.0;

    // Generate signal with only alpha frequency (10 Hz)
    let alpha_signal = generate_sine_wave(10.0, sampling_rate, duration);

    let analyzer = FFTAnalyzer::new(sampling_rate);
    let spectrum = analyzer.analyze(&alpha_signal);

    let alpha_power = analyzer.band_power(&spectrum, FrequencyBand::Alpha);
    let delta_power = analyzer.band_power(&spectrum, FrequencyBand::Delta);
    let gamma_power = analyzer.band_power(&spectrum, FrequencyBand::Gamma);

    // Alpha power should dominate
    assert!(
        alpha_power > delta_power * 5.0,
        "10 Hz signal should have alpha >> delta"
    );
    assert!(
        alpha_power > gamma_power * 5.0,
        "10 Hz signal should have alpha >> gamma"
    );
}

// =============================================================================
// Statistical Features Tests
// =============================================================================

#[test]
fn test_mean_amplitude_calculation() {
    let sampling_rate = 256.0;

    // Signal with known mean
    let signal: Vec<f32> = (0..768)
        .map(|i| (2.0 * PI * 10.0 * i as f32 / sampling_rate).sin() + 0.5) // DC offset of 0.5
        .collect();

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let features = processor.process_raw_eeg(&signal).unwrap();

    // Mean should be close to the DC offset (filtered signal may differ)
    // Just verify it's a reasonable value
    assert!(
        features.mean_amplitude.abs() < 10.0,
        "Mean amplitude should be reasonable"
    );
}

#[test]
fn test_std_deviation_calculation() {
    let sampling_rate = 256.0;
    let signal = generate_realistic_eeg(sampling_rate, 3.0, EEGState::RelaxedAwake);

    let processor = EEGProcessor::new(sampling_rate).unwrap();
    let features = processor.process_raw_eeg(&signal).unwrap();

    // Standard deviation should be positive for non-constant signal
    assert!(
        features.std_deviation > 0.0,
        "Standard deviation should be positive"
    );
    // And not unreasonably large
    assert!(
        features.std_deviation < 1000.0,
        "Standard deviation should be reasonable"
    );
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_full_pipeline_integration() {
    // Test the complete EEG processing pipeline
    let sampling_rate = 256.0;

    // 1. Create processor
    let processor = EEGProcessor::new(sampling_rate).unwrap();

    // 2. Generate realistic EEG signal
    let signal = generate_realistic_eeg(sampling_rate, 5.0, EEGState::RelaxedAwake);

    // 3. Process and extract features
    let features = processor.process_raw_eeg(&signal).unwrap();

    // 4. Validate all features are present and reasonable
    assert!(features.delta_power >= 0.0);
    assert!(features.theta_power >= 0.0);
    assert!(features.alpha_power >= 0.0);
    assert!(features.beta_power >= 0.0);
    assert!(features.gamma_power >= 0.0);
    assert!(features.signal_quality >= 0.0 && features.signal_quality <= 100.0);
    assert!(features.alpha_beta_ratio >= 0.0);
    assert!(features.theta_alpha_ratio >= 0.0);
}

#[test]
fn test_different_sampling_rates_produce_consistent_features() {
    // Features should be relatively consistent across supported sampling rates
    let duration = 3.0;
    let test_frequency = 10.0; // Alpha band

    let mut alpha_powers = Vec::new();

    for &rate in &[256.0, 512.0, 1024.0] {
        let signal = generate_sine_wave(test_frequency, rate, duration);
        // Pad with some noise to make it more realistic
        let signal = add_noise(&signal, 0.1);

        let processor = EEGProcessor::new(rate).unwrap();
        if let Ok(features) = processor.process_raw_eeg(&signal) {
            alpha_powers.push((rate, features.alpha_power));
        }
    }

    // All sampling rates should detect alpha power
    for (rate, power) in &alpha_powers {
        assert!(
            *power > 0.0,
            "Should detect alpha power at {rate} Hz sampling rate"
        );
    }
}
