//! OpenTelemetry distributed tracing setup and configuration

use crate::config::{TracingConfig, TracingExporter, TraceLevel};
use anyhow::{Context, Result};
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, Tracer};
use opentelemetry_sdk::{runtime, Resource};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

/// Initialize OpenTelemetry tracing based on configuration
///
/// Sets up the OpenTelemetry tracer with the configured exporter (Jaeger, OTLP, etc.)
/// and integrates it with the tracing subscriber.
pub fn init_tracing(config: &TracingConfig) -> Result<()> {
    if !config.enabled {
        info!("📊 Distributed tracing is disabled");
        return Ok(());
    }

    info!("📊 Initializing OpenTelemetry distributed tracing");
    info!("   Service: {}", config.service_name);
    info!("   Exporter: {:?}", config.exporter);
    info!("   Endpoint: {}", config.endpoint);
    info!("   Sampling rate: {:.1}%", config.sampling_rate * 100.0);
    info!("   Trace level: {:?}", config.trace_level);

    // Create tracer based on exporter type
    let tracer = match &config.exporter {
        TracingExporter::Jaeger => create_jaeger_tracer(config)?,
        TracingExporter::Otlp => create_otlp_tracer(config)?,
        TracingExporter::Console => create_console_tracer(config)?,
        TracingExporter::Zipkin => {
            anyhow::bail!("Zipkin exporter is not yet implemented");
        }
    };

    // Create the tracing layer with OpenTelemetry
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Set up the subscriber with both logging and tracing
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .context("Failed to create EnvFilter")?;

    let subscriber = Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer);

    subscriber
        .try_init()
        .context("Failed to initialize tracing subscriber")?;

    info!("✅ OpenTelemetry tracing initialized successfully");
    Ok(())
}

/// Default OTLP gRPC endpoint for Jaeger
const DEFAULT_JAEGER_OTLP_ENDPOINT: &str = "http://localhost:4317";

/// Create Jaeger tracer using OTLP gRPC protocol
fn create_jaeger_tracer(config: &TracingConfig) -> Result<Tracer> {
    info!("🔧 Setting up Jaeger via OTLP exporter");

    // Jaeger supports OTLP natively now - use OTLP gRPC endpoint
    // Default Jaeger OTLP endpoint is port 4317
    let otlp_endpoint = if config.endpoint.contains("14268") {
        // Convert old HTTP collector endpoint to OTLP gRPC endpoint
        config.endpoint.replace("14268", "4317").replace("/api/traces", "")
    } else if config.endpoint.contains("4317") {
        config.endpoint.clone()
    } else {
        // Default to OTLP gRPC port
        DEFAULT_JAEGER_OTLP_ENDPOINT.to_string()
    };

    info!("   Using OTLP endpoint: {}", otlp_endpoint);

    // Create resource with service information
    let resource = create_resource(config);

    // Create sampler based on sampling rate
    let sampler = create_sampler(config.sampling_rate);

    // Build OTLP pipeline targeting Jaeger
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&otlp_endpoint),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .install_batch(runtime::Tokio)
        .context("Failed to install OTLP tracer for Jaeger")?;

    Ok(tracer)
}

/// Create OTLP tracer with gRPC exporter
fn create_otlp_tracer(config: &TracingConfig) -> Result<Tracer> {
    info!("🔧 Setting up OTLP gRPC exporter");

    // Create resource with service information
    let resource = create_resource(config);

    // Create sampler based on sampling rate
    let sampler = create_sampler(config.sampling_rate);

    // Build OTLP pipeline
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.endpoint),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .install_batch(runtime::Tokio)
        .context("Failed to install OTLP tracer")?;

    Ok(tracer)
}

/// Create console tracer for development/debugging
fn create_console_tracer(config: &TracingConfig) -> Result<Tracer> {
    info!("🔧 Setting up console exporter (development mode)");

    // Create resource with service information
    let resource = create_resource(config);

    // Create sampler based on sampling rate
    let sampler = create_sampler(config.sampling_rate);

    // Build console/stdout pipeline
    let tracer = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
        .build()
        .tracer("neuroquantumdb");

    Ok(tracer)
}

/// Create OpenTelemetry resource with service information
fn create_resource(config: &TracingConfig) -> Resource {
    let mut attributes = vec![
        KeyValue::new(SERVICE_NAME, config.service_name.clone()),
        KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        KeyValue::new("service.namespace", "neuroquantumdb"),
    ];

    // Add custom resource attributes if provided
    if let Some(custom_attrs) = &config.resource_attributes {
        for (key, value) in custom_attrs {
            attributes.push(KeyValue::new(key.clone(), value.clone()));
        }
    }

    Resource::new(attributes)
}

/// Create sampler based on sampling rate
fn create_sampler(sampling_rate: f64) -> Sampler {
    // Clamp sampling rate between 0.0 and 1.0
    let rate = sampling_rate.clamp(0.0, 1.0);

    if rate >= 1.0 {
        // Always sample
        Sampler::AlwaysOn
    } else if rate <= 0.0 {
        // Never sample
        Sampler::AlwaysOff
    } else {
        // Probabilistic sampling
        Sampler::TraceIdRatioBased(rate)
    }
}

/// Shutdown OpenTelemetry tracing gracefully
pub fn shutdown_tracing() {
    info!("📊 Shutting down OpenTelemetry tracing");
    global::shutdown_tracer_provider();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sampler_always_on() {
        let sampler = create_sampler(1.0);
        assert!(matches!(sampler, Sampler::AlwaysOn));
    }

    #[test]
    fn test_create_sampler_always_off() {
        let sampler = create_sampler(0.0);
        assert!(matches!(sampler, Sampler::AlwaysOff));
    }

    #[test]
    fn test_create_sampler_probabilistic() {
        let sampler = create_sampler(0.5);
        assert!(matches!(sampler, Sampler::TraceIdRatioBased(_)));
    }

    #[test]
    fn test_create_sampler_clamps_high() {
        let sampler = create_sampler(2.0);
        assert!(matches!(sampler, Sampler::AlwaysOn));
    }

    #[test]
    fn test_create_sampler_clamps_low() {
        let sampler = create_sampler(-1.0);
        assert!(matches!(sampler, Sampler::AlwaysOff));
    }

    #[test]
    fn test_create_resource() {
        let config = TracingConfig::default();
        let resource = create_resource(&config);
        
        // Check that resource has required attributes
        let attrs: Vec<_> = resource.iter().collect();
        assert!(!attrs.is_empty());
    }
}
