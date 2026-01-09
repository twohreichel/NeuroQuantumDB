//! Cluster metrics for monitoring and observability.
//!
//! This module provides Prometheus-compatible metrics for monitoring
//! cluster health, performance, and operations.

use std::sync::atomic::{AtomicU64, Ordering};

use tracing::debug;

use crate::node::{ClusterHealth, NodeId};
use crate::sharding::RebalanceProgress;

/// Cluster metrics collection.
pub struct ClusterMetrics {
    /// Node ID for this metrics instance
    node_id: NodeId,
    /// Total proposals made
    proposals_total: AtomicU64,
    /// Total successful proposals
    proposals_success: AtomicU64,
    /// Total failed proposals
    proposals_failed: AtomicU64,
    /// Total health checks performed
    health_checks_total: AtomicU64,
    /// Total elections started
    elections_started: AtomicU64,
    /// Total leader elections won
    elections_won: AtomicU64,
    /// Total messages sent
    messages_sent: AtomicU64,
    /// Total messages received
    messages_received: AtomicU64,
    /// Total bytes sent
    bytes_sent: AtomicU64,
    /// Total bytes received
    bytes_received: AtomicU64,
    /// Current peer count
    peer_count: AtomicU64,
    /// Current healthy peer count
    healthy_peer_count: AtomicU64,
    /// Node start time (Unix timestamp in ms)
    start_time_ms: AtomicU64,
    /// Last heartbeat received time
    last_heartbeat_ms: AtomicU64,
    /// Replication lag in ms
    replication_lag_ms: AtomicU64,
    /// Rebalancing active flag (1 = active, 0 = inactive)
    rebalancing_active: AtomicU64,
    /// Total shard transfers
    rebalance_transfers_total: AtomicU64,
    /// Completed shard transfers
    rebalance_transfers_completed: AtomicU64,
    /// Failed shard transfers
    rebalance_transfers_failed: AtomicU64,
    /// Bytes transferred during rebalancing
    rebalance_bytes_transferred: AtomicU64,
    /// Rebalancing throughput (bytes/sec)
    rebalance_throughput: AtomicU64,
}

impl ClusterMetrics {
    /// Create a new metrics instance for the given node.
    #[must_use]
    pub fn new(node_id: NodeId) -> Self {
        debug!(node_id, "Creating cluster metrics");

        Self {
            node_id,
            proposals_total: AtomicU64::new(0),
            proposals_success: AtomicU64::new(0),
            proposals_failed: AtomicU64::new(0),
            health_checks_total: AtomicU64::new(0),
            elections_started: AtomicU64::new(0),
            elections_won: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            peer_count: AtomicU64::new(0),
            healthy_peer_count: AtomicU64::new(0),
            start_time_ms: AtomicU64::new(0),
            last_heartbeat_ms: AtomicU64::new(0),
            replication_lag_ms: AtomicU64::new(0),
            rebalancing_active: AtomicU64::new(0),
            rebalance_transfers_total: AtomicU64::new(0),
            rebalance_transfers_completed: AtomicU64::new(0),
            rebalance_transfers_failed: AtomicU64::new(0),
            rebalance_bytes_transferred: AtomicU64::new(0),
            rebalance_throughput: AtomicU64::new(0),
        }
    }

    /// Record that the node has started.
    pub fn record_start(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.start_time_ms.store(now, Ordering::Relaxed);
    }

    /// Record a proposal attempt.
    pub fn record_proposal(&self) {
        self.proposals_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful proposal.
    pub fn record_proposal_success(&self) {
        self.proposals_success.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed proposal.
    pub fn record_proposal_failure(&self) {
        self.proposals_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a health check.
    pub fn record_health_check(&self, health: &ClusterHealth) {
        self.health_checks_total.fetch_add(1, Ordering::Relaxed);
        self.peer_count
            .store(health.total_peers as u64, Ordering::Relaxed);
        self.healthy_peer_count
            .store(health.healthy_peers as u64, Ordering::Relaxed);
    }

    /// Record an election start.
    pub fn record_election_start(&self) {
        self.elections_started.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an election win.
    pub fn record_election_win(&self) {
        self.elections_won.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a message sent.
    pub fn record_message_sent(&self, bytes: u64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a message received.
    pub fn record_message_received(&self, bytes: u64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a heartbeat received.
    pub fn record_heartbeat(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.last_heartbeat_ms.store(now, Ordering::Relaxed);
    }

    /// Record replication lag.
    pub fn record_replication_lag(&self, lag_ms: u64) {
        self.replication_lag_ms.store(lag_ms, Ordering::Relaxed);
    }

    /// Update rebalance progress metrics.
    pub fn update_rebalance_progress(&self, progress: &RebalanceProgress) {
        self.rebalancing_active
            .store(if progress.active { 1 } else { 0 }, Ordering::Relaxed);
        self.rebalance_transfers_total
            .store(progress.total_transfers as u64, Ordering::Relaxed);
        self.rebalance_transfers_completed
            .store(progress.completed_transfers as u64, Ordering::Relaxed);
        self.rebalance_transfers_failed
            .store(progress.failed_transfers as u64, Ordering::Relaxed);
        self.rebalance_bytes_transferred
            .store(progress.bytes_transferred, Ordering::Relaxed);
        self.rebalance_throughput
            .store(progress.throughput_bytes_per_sec, Ordering::Relaxed);
    }

    /// Record a completed transfer.
    pub fn record_transfer_complete(&self) {
        self.rebalance_transfers_completed
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed transfer.
    pub fn record_transfer_failed(&self) {
        self.rebalance_transfers_failed
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get all metrics as a snapshot.
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let start_time = self.start_time_ms.load(Ordering::Relaxed);
        let uptime_secs = if start_time > 0 {
            (now - start_time) / 1000
        } else {
            0
        };

        MetricsSnapshot {
            node_id: self.node_id,
            uptime_secs,
            proposals_total: self.proposals_total.load(Ordering::Relaxed),
            proposals_success: self.proposals_success.load(Ordering::Relaxed),
            proposals_failed: self.proposals_failed.load(Ordering::Relaxed),
            health_checks_total: self.health_checks_total.load(Ordering::Relaxed),
            elections_started: self.elections_started.load(Ordering::Relaxed),
            elections_won: self.elections_won.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            peer_count: self.peer_count.load(Ordering::Relaxed),
            healthy_peer_count: self.healthy_peer_count.load(Ordering::Relaxed),
            last_heartbeat_ms: self.last_heartbeat_ms.load(Ordering::Relaxed),
            replication_lag_ms: self.replication_lag_ms.load(Ordering::Relaxed),
            rebalancing_active: self.rebalancing_active.load(Ordering::Relaxed) == 1,
            rebalance_transfers_total: self.rebalance_transfers_total.load(Ordering::Relaxed),
            rebalance_transfers_completed: self
                .rebalance_transfers_completed
                .load(Ordering::Relaxed),
            rebalance_transfers_failed: self.rebalance_transfers_failed.load(Ordering::Relaxed),
            rebalance_bytes_transferred: self.rebalance_bytes_transferred.load(Ordering::Relaxed),
            rebalance_throughput_bytes_per_sec: self.rebalance_throughput.load(Ordering::Relaxed),
        }
    }

    /// Export metrics in Prometheus format.
    #[must_use]
    pub fn to_prometheus(&self) -> String {
        let snapshot = self.snapshot();

        format!(
            r#"# HELP neuroquantum_cluster_uptime_seconds Node uptime in seconds
# TYPE neuroquantum_cluster_uptime_seconds gauge
neuroquantum_cluster_uptime_seconds{{node_id="{node_id}"}} {uptime}

# HELP neuroquantum_cluster_proposals_total Total number of proposals
# TYPE neuroquantum_cluster_proposals_total counter
neuroquantum_cluster_proposals_total{{node_id="{node_id}"}} {proposals_total}

# HELP neuroquantum_cluster_proposals_success_total Total successful proposals
# TYPE neuroquantum_cluster_proposals_success_total counter
neuroquantum_cluster_proposals_success_total{{node_id="{node_id}"}} {proposals_success}

# HELP neuroquantum_cluster_proposals_failed_total Total failed proposals
# TYPE neuroquantum_cluster_proposals_failed_total counter
neuroquantum_cluster_proposals_failed_total{{node_id="{node_id}"}} {proposals_failed}

# HELP neuroquantum_cluster_elections_started_total Total elections started
# TYPE neuroquantum_cluster_elections_started_total counter
neuroquantum_cluster_elections_started_total{{node_id="{node_id}"}} {elections_started}

# HELP neuroquantum_cluster_elections_won_total Total elections won
# TYPE neuroquantum_cluster_elections_won_total counter
neuroquantum_cluster_elections_won_total{{node_id="{node_id}"}} {elections_won}

# HELP neuroquantum_cluster_messages_sent_total Total messages sent
# TYPE neuroquantum_cluster_messages_sent_total counter
neuroquantum_cluster_messages_sent_total{{node_id="{node_id}"}} {messages_sent}

# HELP neuroquantum_cluster_messages_received_total Total messages received
# TYPE neuroquantum_cluster_messages_received_total counter
neuroquantum_cluster_messages_received_total{{node_id="{node_id}"}} {messages_received}

# HELP neuroquantum_cluster_bytes_sent_total Total bytes sent
# TYPE neuroquantum_cluster_bytes_sent_total counter
neuroquantum_cluster_bytes_sent_total{{node_id="{node_id}"}} {bytes_sent}

# HELP neuroquantum_cluster_bytes_received_total Total bytes received
# TYPE neuroquantum_cluster_bytes_received_total counter
neuroquantum_cluster_bytes_received_total{{node_id="{node_id}"}} {bytes_received}

# HELP neuroquantum_cluster_peer_count Current number of peers
# TYPE neuroquantum_cluster_peer_count gauge
neuroquantum_cluster_peer_count{{node_id="{node_id}"}} {peer_count}

# HELP neuroquantum_cluster_healthy_peer_count Current number of healthy peers
# TYPE neuroquantum_cluster_healthy_peer_count gauge
neuroquantum_cluster_healthy_peer_count{{node_id="{node_id}"}} {healthy_peer_count}

# HELP neuroquantum_cluster_replication_lag_ms Current replication lag in milliseconds
# TYPE neuroquantum_cluster_replication_lag_ms gauge
neuroquantum_cluster_replication_lag_ms{{node_id="{node_id}"}} {replication_lag}

# HELP neuroquantum_cluster_rebalancing_active Whether shard rebalancing is active
# TYPE neuroquantum_cluster_rebalancing_active gauge
neuroquantum_cluster_rebalancing_active{{node_id="{node_id}"}} {rebalancing_active}

# HELP neuroquantum_cluster_rebalance_transfers_total Total shard transfers in current rebalance
# TYPE neuroquantum_cluster_rebalance_transfers_total gauge
neuroquantum_cluster_rebalance_transfers_total{{node_id="{node_id}"}} {rebalance_transfers_total}

# HELP neuroquantum_cluster_rebalance_transfers_completed Completed shard transfers
# TYPE neuroquantum_cluster_rebalance_transfers_completed counter
neuroquantum_cluster_rebalance_transfers_completed{{node_id="{node_id}"}} {rebalance_transfers_completed}

# HELP neuroquantum_cluster_rebalance_transfers_failed Failed shard transfers
# TYPE neuroquantum_cluster_rebalance_transfers_failed counter
neuroquantum_cluster_rebalance_transfers_failed{{node_id="{node_id}"}} {rebalance_transfers_failed}

# HELP neuroquantum_cluster_rebalance_bytes_transferred Bytes transferred during rebalancing
# TYPE neuroquantum_cluster_rebalance_bytes_transferred counter
neuroquantum_cluster_rebalance_bytes_transferred{{node_id="{node_id}"}} {rebalance_bytes_transferred}

# HELP neuroquantum_cluster_rebalance_throughput_bytes_per_sec Rebalancing throughput in bytes per second
# TYPE neuroquantum_cluster_rebalance_throughput_bytes_per_sec gauge
neuroquantum_cluster_rebalance_throughput_bytes_per_sec{{node_id="{node_id}"}} {rebalance_throughput}
"#,
            node_id = snapshot.node_id,
            uptime = snapshot.uptime_secs,
            proposals_total = snapshot.proposals_total,
            proposals_success = snapshot.proposals_success,
            proposals_failed = snapshot.proposals_failed,
            elections_started = snapshot.elections_started,
            elections_won = snapshot.elections_won,
            messages_sent = snapshot.messages_sent,
            messages_received = snapshot.messages_received,
            bytes_sent = snapshot.bytes_sent,
            bytes_received = snapshot.bytes_received,
            peer_count = snapshot.peer_count,
            healthy_peer_count = snapshot.healthy_peer_count,
            replication_lag = snapshot.replication_lag_ms,
            rebalancing_active = if snapshot.rebalancing_active { 1 } else { 0 },
            rebalance_transfers_total = snapshot.rebalance_transfers_total,
            rebalance_transfers_completed = snapshot.rebalance_transfers_completed,
            rebalance_transfers_failed = snapshot.rebalance_transfers_failed,
            rebalance_bytes_transferred = snapshot.rebalance_bytes_transferred,
            rebalance_throughput = snapshot.rebalance_throughput_bytes_per_sec,
        )
    }
}

/// Snapshot of cluster metrics at a point in time.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    /// Node ID
    pub node_id: NodeId,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Total proposals made
    pub proposals_total: u64,
    /// Total successful proposals
    pub proposals_success: u64,
    /// Total failed proposals
    pub proposals_failed: u64,
    /// Total health checks performed
    pub health_checks_total: u64,
    /// Total elections started
    pub elections_started: u64,
    /// Total elections won
    pub elections_won: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Current peer count
    pub peer_count: u64,
    /// Current healthy peer count
    pub healthy_peer_count: u64,
    /// Last heartbeat time (Unix timestamp in ms)
    pub last_heartbeat_ms: u64,
    /// Replication lag in ms
    pub replication_lag_ms: u64,
    /// Whether rebalancing is active
    pub rebalancing_active: bool,
    /// Total shard transfers in current rebalance
    pub rebalance_transfers_total: u64,
    /// Completed shard transfers
    pub rebalance_transfers_completed: u64,
    /// Failed shard transfers
    pub rebalance_transfers_failed: u64,
    /// Bytes transferred during rebalancing
    pub rebalance_bytes_transferred: u64,
    /// Rebalancing throughput (bytes/sec)
    pub rebalance_throughput_bytes_per_sec: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = ClusterMetrics::new(1);
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.node_id, 1);
        assert_eq!(snapshot.proposals_total, 0);
    }

    #[test]
    fn test_record_start() {
        let metrics = ClusterMetrics::new(1);
        metrics.record_start();

        let snapshot = metrics.snapshot();
        // Uptime should be 0 or very small (just started)
        assert!(snapshot.uptime_secs <= 1);
    }

    #[test]
    fn test_record_proposal() {
        let metrics = ClusterMetrics::new(1);

        // Record two proposal attempts
        metrics.record_proposal();
        metrics.record_proposal();
        // Record outcomes: one success, one failure
        metrics.record_proposal_success();
        metrics.record_proposal_failure();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.proposals_total, 2);
        assert_eq!(snapshot.proposals_success, 1);
        assert_eq!(snapshot.proposals_failed, 1);
    }

    #[test]
    fn test_record_messages() {
        let metrics = ClusterMetrics::new(1);

        metrics.record_message_sent(100);
        metrics.record_message_sent(200);
        metrics.record_message_received(50);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.messages_sent, 2);
        assert_eq!(snapshot.bytes_sent, 300);
        assert_eq!(snapshot.messages_received, 1);
        assert_eq!(snapshot.bytes_received, 50);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = ClusterMetrics::new(42);
        metrics.record_start();
        metrics.record_proposal();

        let prometheus = metrics.to_prometheus();

        assert!(prometheus.contains("neuroquantum_cluster_proposals_total{node_id=\"42\"} 1"));
        assert!(prometheus.contains("neuroquantum_cluster_uptime_seconds{node_id=\"42\"}"));
    }

    #[test]
    fn test_record_elections() {
        let metrics = ClusterMetrics::new(1);

        metrics.record_election_start();
        metrics.record_election_start();
        metrics.record_election_win();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.elections_started, 2);
        assert_eq!(snapshot.elections_won, 1);
    }
}
