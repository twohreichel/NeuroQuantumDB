//! Unit tests for rolling upgrades.

use neuroquantum_cluster::upgrade::{UpgradeCoordinator, UpgradeStatus};

#[tokio::test]
async fn test_upgrade_coordinator_creation() {
    let coordinator = UpgradeCoordinator::new(3);
    assert_eq!(coordinator.status().await, UpgradeStatus::Idle);

    let progress = coordinator.progress().await;
    assert_eq!(progress.total_nodes, 3);
    assert_eq!(progress.upgraded_nodes, 0);
}

#[tokio::test]
async fn test_upgrade_status_display() {
    assert_eq!(format!("{}", UpgradeStatus::Idle), "Idle");
    assert_eq!(format!("{}", UpgradeStatus::Preparing), "Preparing");
    assert_eq!(format!("{}", UpgradeStatus::Completed), "Completed");
    assert_eq!(format!("{}", UpgradeStatus::Failed), "Failed");
}

#[tokio::test]
async fn test_coordinator_reset() {
    let coordinator = UpgradeCoordinator::new(3);

    // Simulate upgrade in progress
    coordinator.set_status(UpgradeStatus::Preparing).await;

    coordinator.reset().await;
    assert_eq!(coordinator.status().await, UpgradeStatus::Idle);
}
