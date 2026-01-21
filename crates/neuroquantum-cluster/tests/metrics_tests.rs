//! Unit tests for cluster metrics.

use neuroquantum_cluster::metrics::ClusterMetrics;

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
