//! Service discovery for cluster nodes.

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::config::{ClusterConfig, DiscoveryMethod};
use crate::error::{ClusterError, ClusterResult};
use crate::node::{NodeId, NodeRole, PeerInfo};

/// Service discovery for finding cluster nodes.
pub struct DiscoveryService {
    /// Discovery method
    method: DiscoveryMethod,
    /// Static nodes (if using static discovery)
    static_nodes: Vec<String>,
    /// DNS name for DNS-based discovery
    dns_name: Option<String>,
    /// Local node ID (to exclude from discovery results)
    local_node_id: NodeId,
}

impl DiscoveryService {
    /// Create a new discovery service.
    pub fn new(config: &ClusterConfig) -> ClusterResult<Self> {
        info!(
            node_id = config.node_id,
            method = ?config.discovery.method,
            "Creating discovery service"
        );

        Ok(Self {
            method: config.discovery.method.clone(),
            static_nodes: config.discovery.static_nodes.clone(),
            dns_name: config.discovery.dns_name.clone(),
            local_node_id: config.node_id,
        })
    }

    /// Discover cluster nodes.
    pub async fn discover(&self) -> ClusterResult<Vec<PeerInfo>> {
        match &self.method {
            DiscoveryMethod::Static => self.discover_static().await,
            DiscoveryMethod::Dns => self.discover_dns().await,
            DiscoveryMethod::Consul => self.discover_consul().await,
            DiscoveryMethod::Etcd => self.discover_etcd().await,
        }
    }

    /// Discover nodes using static configuration.
    async fn discover_static(&self) -> ClusterResult<Vec<PeerInfo>> {
        debug!("Using static node discovery");

        let mut peers = Vec::new();

        for (idx, node_addr) in self.static_nodes.iter().enumerate() {
            // Parse address
            let addr: SocketAddr = node_addr.parse().map_err(|e| {
                ClusterError::ConfigError(format!("Invalid node address '{}': {}", node_addr, e))
            })?;

            // Generate a deterministic node ID based on position
            // In production, node IDs would be exchanged during handshake
            let node_id = (idx + 1) as u64;

            // Skip self
            if node_id == self.local_node_id {
                continue;
            }

            peers.push(PeerInfo {
                node_id,
                addr,
                role: NodeRole::Follower, // Unknown at discovery time
                last_heartbeat: None,
                healthy: false, // Will be updated after connection
            });
        }

        info!(count = peers.len(), "Discovered nodes via static config");
        Ok(peers)
    }

    /// Discover nodes using DNS (e.g., Kubernetes headless service).
    async fn discover_dns(&self) -> ClusterResult<Vec<PeerInfo>> {
        let dns_name = self.dns_name.as_ref().ok_or_else(|| {
            ClusterError::ConfigError("DNS discovery enabled but no dns_name configured".into())
        })?;

        debug!(dns_name, "Using DNS node discovery");

        // In a full implementation:
        // 1. Use trust-dns-resolver to query the DNS name
        // 2. Get all A/AAAA records
        // 3. Connect to each to exchange node IDs

        warn!(dns_name, "DNS discovery not yet fully implemented");
        Ok(Vec::new())
    }

    /// Discover nodes using Consul service discovery.
    async fn discover_consul(&self) -> ClusterResult<Vec<PeerInfo>> {
        debug!("Using Consul node discovery");

        // In a full implementation:
        // 1. Connect to Consul agent
        // 2. Query for service nodes
        // 3. Return node information

        warn!("Consul discovery not yet implemented");
        Ok(Vec::new())
    }

    /// Discover nodes using etcd.
    async fn discover_etcd(&self) -> ClusterResult<Vec<PeerInfo>> {
        debug!("Using etcd node discovery");

        // In a full implementation:
        // 1. Connect to etcd cluster
        // 2. Query for registered nodes
        // 3. Watch for changes

        warn!("etcd discovery not yet implemented");
        Ok(Vec::new())
    }
}

/// Node registration for service discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistration {
    /// Node identifier
    pub node_id: NodeId,
    /// Node address
    pub address: String,
    /// Node metadata
    pub metadata: NodeMetadata,
    /// Registration time (Unix timestamp)
    pub registered_at: u64,
    /// TTL for the registration
    pub ttl_secs: u64,
}

/// Metadata about a node for service discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    /// Version of NeuroQuantumDB running on this node
    pub version: String,
    /// Datacenter or zone
    pub datacenter: Option<String>,
    /// Additional tags
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_static_discovery() {
        let config = ClusterConfig {
            node_id: 1,
            discovery: crate::config::DiscoveryConfig {
                method: DiscoveryMethod::Static,
                static_nodes: vec![
                    "127.0.0.1:9001".into(),
                    "127.0.0.1:9002".into(),
                    "127.0.0.1:9003".into(),
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        let service = DiscoveryService::new(&config).unwrap();
        let peers = service.discover().await.unwrap();

        // Should discover 2 peers (excluding self with node_id 1)
        assert_eq!(peers.len(), 2);
    }

    #[test]
    fn test_node_registration_serialization() {
        let reg = NodeRegistration {
            node_id: 1,
            address: "127.0.0.1:9000".into(),
            metadata: NodeMetadata {
                version: "0.1.0".into(),
                datacenter: Some("us-east-1".into()),
                tags: vec!["primary".into()],
            },
            registered_at: 1234567890,
            ttl_secs: 30,
        };

        let json = serde_json::to_string(&reg).unwrap();
        let parsed: NodeRegistration = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.node_id, 1);
        assert_eq!(parsed.metadata.version, "0.1.0");
    }
}
