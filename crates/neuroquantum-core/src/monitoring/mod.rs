//! Monitoring and Observability Module
//!
//! Provides comprehensive monitoring capabilities including:
//! - Query metrics and slow query logging
//! - Prometheus metrics export
//! - Performance tracking

pub mod prometheus;
pub mod query_metrics;

pub use prometheus::MetricsExporter;
pub use query_metrics::{
    AdvancedQueryMetrics, MonitoringConfig, QueryExecutionParams, SlowQueryEntry,
};
