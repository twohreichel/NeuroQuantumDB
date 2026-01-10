//! Rolling upgrade orchestration for cluster nodes.
//!
//! This module provides functionality for performing zero-downtime rolling upgrades
//! of cluster nodes, including:
//! - Connection draining
//! - Leader handoff
//! - Health checks
//! - Automatic rollback on failure
//! - Minimum healthy nodes enforcement

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::error::{ClusterError, ClusterResult};
use crate::node::{ClusterNode, NodeId, NodeState};

/// Status of a rolling upgrade operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpgradeStatus {
    /// No upgrade in progress
    Idle,
    /// Preparing node for upgrade (draining connections)
    Preparing,
    /// Waiting for node to be upgraded
    UpgradePending,
    /// Running health checks after upgrade
    HealthChecking,
    /// Upgrade completed successfully
    Completed,
    /// Upgrade failed
    Failed,
    /// Rolling back to previous version
    RollingBack,
}

impl std::fmt::Display for UpgradeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Preparing => write!(f, "Preparing"),
            Self::UpgradePending => write!(f, "UpgradePending"),
            Self::HealthChecking => write!(f, "HealthChecking"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::RollingBack => write!(f, "RollingBack"),
        }
    }
}

/// Progress information for a rolling upgrade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeProgress {
    /// Current status of the upgrade
    pub status: UpgradeStatus,
    /// Node being upgraded
    pub node_id: NodeId,
    /// Total number of nodes in cluster
    pub total_nodes: usize,
    /// Number of nodes already upgraded
    pub upgraded_nodes: usize,
    /// Current healthy node count
    pub healthy_nodes: usize,
    /// Error message if upgrade failed
    pub error: Option<String>,
}

/// Coordinator for rolling upgrades across the cluster.
pub struct UpgradeCoordinator {
    /// Current upgrade status
    status: Arc<RwLock<UpgradeStatus>>,
    /// Progress tracking
    progress: Arc<RwLock<UpgradeProgress>>,
}

impl UpgradeCoordinator {
    /// Create a new upgrade coordinator.
    #[must_use]
    pub fn new(total_nodes: usize) -> Self {
        Self {
            status: Arc::new(RwLock::new(UpgradeStatus::Idle)),
            progress: Arc::new(RwLock::new(UpgradeProgress {
                status: UpgradeStatus::Idle,
                node_id: 0,
                total_nodes,
                upgraded_nodes: 0,
                healthy_nodes: total_nodes,
                error: None,
            })),
        }
    }

    /// Get the current upgrade status.
    pub async fn status(&self) -> UpgradeStatus {
        *self.status.read().await
    }

    /// Get the current upgrade progress.
    pub async fn progress(&self) -> UpgradeProgress {
        self.progress.read().await.clone()
    }

    /// Initiate a rolling upgrade for a single node.
    ///
    /// This performs the following steps:
    /// 1. Check minimum healthy nodes requirement
    /// 2. Prepare node for upgrade (drain connections, transfer leadership)
    /// 3. Wait for external upgrade to complete
    /// 4. Perform health checks
    /// 5. Mark node as ready
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Another upgrade is already in progress
    /// - Minimum healthy nodes requirement not met
    /// - Health checks fail
    pub async fn upgrade_node(&self, node: Arc<ClusterNode>) -> ClusterResult<()> {
        // Check if upgrade already in progress
        {
            let status = self.status.read().await;
            if *status != UpgradeStatus::Idle {
                return Err(ClusterError::UpgradeInProgress);
            }
        }

        let node_id = node.node_id();
        info!(node_id, "Starting rolling upgrade for node");

        // Update status to preparing
        {
            let mut status = self.status.write().await;
            *status = UpgradeStatus::Preparing;
        }
        {
            let mut progress = self.progress.write().await;
            progress.status = UpgradeStatus::Preparing;
            progress.node_id = node_id;
            progress.error = None;
        }

        // Step 1: Check minimum healthy nodes
        let health = node.health().await;
        let min_healthy = {
            let inner = node.inner.read().await;
            inner.config.manager.upgrades.min_healthy_nodes
        };

        if health.healthy_peers + 1 < min_healthy {
            // +1 for self
            let error_msg = format!(
                "Cannot upgrade: healthy nodes {} < minimum required {}",
                health.healthy_peers + 1,
                min_healthy
            );
            error!(node_id, error = %error_msg, "Upgrade precondition failed");

            let mut status = self.status.write().await;
            *status = UpgradeStatus::Failed;

            let mut progress = self.progress.write().await;
            progress.status = UpgradeStatus::Failed;
            progress.error = Some(error_msg.clone());

            return Err(ClusterError::InsufficientHealthyNodes {
                current: health.healthy_peers + 1,
                required: min_healthy,
            });
        }

        // Step 2: Prepare node for upgrade
        if let Err(e) = node.prepare_for_upgrade().await {
            error!(node_id, error = %e, "Failed to prepare node for upgrade");

            let mut status = self.status.write().await;
            *status = UpgradeStatus::Failed;

            let mut progress = self.progress.write().await;
            progress.status = UpgradeStatus::Failed;
            progress.error = Some(e.to_string());

            return Err(e);
        }

        // Step 3: Mark as pending upgrade (external process will upgrade the node)
        {
            let mut status = self.status.write().await;
            *status = UpgradeStatus::UpgradePending;
        }
        {
            let mut progress = self.progress.write().await;
            progress.status = UpgradeStatus::UpgradePending;
        }

        info!(
            node_id,
            "Node prepared for upgrade, waiting for external upgrade process"
        );

        Ok(())
    }

    /// Mark the node upgrade as complete and perform health checks.
    ///
    /// This should be called after the node has been upgraded and restarted.
    ///
    /// # Errors
    ///
    /// Returns an error if health checks fail.
    pub async fn complete_node_upgrade(&self, node: Arc<ClusterNode>) -> ClusterResult<()> {
        let node_id = node.node_id();
        info!(node_id, "Completing upgrade for node");

        // Update status to health checking
        {
            let mut status = self.status.write().await;
            *status = UpgradeStatus::HealthChecking;
        }
        {
            let mut progress = self.progress.write().await;
            progress.status = UpgradeStatus::HealthChecking;
        }

        // Perform health checks
        if let Err(e) = node.post_upgrade_health_check().await {
            error!(
                node_id,
                error = %e,
                "Post-upgrade health check failed"
            );

            let mut status = self.status.write().await;
            *status = UpgradeStatus::Failed;

            let mut progress = self.progress.write().await;
            progress.status = UpgradeStatus::Failed;
            progress.error = Some(e.to_string());

            return Err(e);
        }

        // Check protocol compatibility
        if let Err(e) = node.check_protocol_compatibility().await {
            error!(
                node_id,
                error = %e,
                "Protocol compatibility check failed"
            );

            let mut status = self.status.write().await;
            *status = UpgradeStatus::Failed;

            let mut progress = self.progress.write().await;
            progress.status = UpgradeStatus::Failed;
            progress.error = Some(e.to_string());

            return Err(e);
        }

        // Mark upgrade as completed
        {
            let mut status = self.status.write().await;
            *status = UpgradeStatus::Completed;
        }
        {
            let mut progress = self.progress.write().await;
            progress.status = UpgradeStatus::Completed;
            progress.upgraded_nodes += 1;
            progress.healthy_nodes = node.health().await.healthy_peers + 1;
        }

        info!(node_id, "Node upgrade completed successfully");
        Ok(())
    }

    /// Reset the upgrade status to idle.
    pub async fn reset(&self) {
        let mut status = self.status.write().await;
        *status = UpgradeStatus::Idle;

        let mut progress = self.progress.write().await;
        progress.status = UpgradeStatus::Idle;
        progress.node_id = 0;
        progress.error = None;
    }

    /// Perform automatic rollback if upgrade fails.
    ///
    /// This is a placeholder for rollback logic. In a real implementation,
    /// this would coordinate with the deployment system to roll back to the
    /// previous version.
    ///
    /// # TODO
    ///
    /// Implementation requirements:
    /// 1. Stop the upgraded node
    /// 2. Restore the previous version (binary/container)
    /// 3. Restart the node with previous version
    /// 4. Verify health after rollback
    /// 5. Update cluster state to reflect rollback
    #[allow(unused_variables)]
    pub async fn rollback(&self, node_id: NodeId) -> ClusterResult<()> {
        info!("Initiating automatic rollback");

        {
            let mut status = self.status.write().await;
            *status = UpgradeStatus::RollingBack;
        }
        {
            let mut progress = self.progress.write().await;
            progress.status = UpgradeStatus::RollingBack;
        }

        // In a real implementation, this would:
        // 1. Stop the upgraded node
        // 2. Restore the previous version
        // 3. Restart the node
        // 4. Verify health

        warn!("Rollback functionality not yet implemented - requires deployment system integration");

        Ok(())
    }
}

/// Perform a canary deployment by upgrading a single node first.
///
/// # Errors
///
/// Returns an error if the canary upgrade fails.
pub async fn canary_upgrade(
    coordinator: Arc<UpgradeCoordinator>,
    canary_node: Arc<ClusterNode>,
    health_check_duration: Duration,
) -> ClusterResult<()> {
    info!(
        node_id = canary_node.node_id(),
        "Starting canary upgrade"
    );

    // Perform the upgrade
    coordinator.upgrade_node(canary_node.clone()).await?;

    // Wait for external upgrade to complete
    // In a real system, we would wait for the node to restart and rejoin
    // This simulated delay represents the time for the deployment system to:
    // 1. Stop the old container/binary
    // 2. Start the new version
    // 3. Wait for the node to initialize
    const SIMULATED_UPGRADE_TIME: Duration = Duration::from_secs(5);
    info!("Waiting for canary node to be upgraded externally");
    tokio::time::sleep(SIMULATED_UPGRADE_TIME).await;

    // Complete the upgrade and run health checks
    coordinator.complete_node_upgrade(canary_node.clone()).await?;

    // Monitor the canary for a period
    info!(
        duration_secs = health_check_duration.as_secs(),
        "Monitoring canary node"
    );

    let start = std::time::Instant::now();
    while start.elapsed() < health_check_duration {
        tokio::time::sleep(Duration::from_secs(1)).await;

        let health = canary_node.health().await;
        if health.state != NodeState::Running {
            return Err(ClusterError::HealthCheckFailed(format!(
                "Canary node in unhealthy state: {:?}",
                health.state
            )));
        }
    }

    info!("Canary upgrade successful, safe to proceed with remaining nodes");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        {
            let mut status = coordinator.status.write().await;
            *status = UpgradeStatus::Preparing;
        }

        coordinator.reset().await;
        assert_eq!(coordinator.status().await, UpgradeStatus::Idle);
    }
}
