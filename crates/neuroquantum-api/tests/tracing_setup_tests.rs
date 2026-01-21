//! Tests for tracing setup and sampler creation
//!
//! These tests validate trace sampler configuration and resource creation.
//!
//! Note: These tests rely on internal APIs exposed for testing purposes.

use neuroquantum_api::config::TracingConfig;
use neuroquantum_api::tracing_setup::{create_resource, create_sampler, SamplerType};

#[test]
fn test_create_sampler_always_on() {
    let sampler = create_sampler(1.0);
    assert!(matches!(sampler, SamplerType::AlwaysOn));
}

#[test]
fn test_create_sampler_always_off() {
    let sampler = create_sampler(0.0);
    assert!(matches!(sampler, SamplerType::AlwaysOff));
}

#[test]
fn test_create_sampler_probabilistic() {
    let sampler = create_sampler(0.5);
    assert!(matches!(sampler, SamplerType::TraceIdRatioBased(_)));
}

#[test]
fn test_create_sampler_clamps_high() {
    let sampler = create_sampler(2.0);
    assert!(matches!(sampler, SamplerType::AlwaysOn));
}

#[test]
fn test_create_sampler_clamps_low() {
    let sampler = create_sampler(-1.0);
    assert!(matches!(sampler, SamplerType::AlwaysOff));
}

#[test]
fn test_create_resource() {
    let config = TracingConfig::default();
    let resource = create_resource(&config);

    // Check that resource has required attributes
    let attrs: Vec<_> = resource.iter().collect();
    assert!(!attrs.is_empty());
}
