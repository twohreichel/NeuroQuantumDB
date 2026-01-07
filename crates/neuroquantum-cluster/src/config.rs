//! Cluster configuration and builder.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{ClusterError, ClusterResult};

/// Configuration for a cluster node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Unique node identifier (must be unique across the cluster)
    pub node_id: u64,

    /// Address to bind for cluster communication
    pub bind_addr: SocketAddr,

    /// Address advertised to other nodes (if different from bind_addr)
    pub advertise_addr: Option<SocketAddr>,

    /// List of peer addresses to connect to on startup
    pub peers: Vec<String>,

    /// Data directory for Raft logs and snapshots
    pub data_dir: PathBuf,

    /// Raft-specific configuration
    pub raft: RaftConfig,

    /// Network configuration
    pub network: NetworkConfig,

    /// Sharding configuration
    pub sharding: ShardingConfig,

    /// Discovery configuration
    pub discovery: DiscoveryConfig,

    /// Cluster manager configuration
    pub manager: ClusterManagerConfig,
}

/// Cluster manager configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterManagerConfig {
    /// Timeout for replication operations
    pub replication_timeout: Duration,

    /// Interval for health monitoring checks
    pub health_check_interval: Duration,

    /// Interval for replication cleanup
    pub replication_cleanup_interval: Duration,
}

impl Default for ClusterManagerConfig {
    fn default() -> Self {
        Self {
            replication_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(5),
            replication_cleanup_interval: Duration::from_secs(60),
        }
    }
}

/// Raft consensus configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    /// Heartbeat interval for leader to send to followers
    pub heartbeat_interval: Duration,

    /// Election timeout minimum (randomized between min and max)
    pub election_timeout_min: Duration,

    /// Election timeout maximum
    pub election_timeout_max: Duration,

    /// Maximum number of entries per append entries RPC
    pub max_entries_per_rpc: u64,

    /// Snapshot threshold (create snapshot after this many log entries)
    pub snapshot_threshold: u64,

    /// Maximum size of snapshot chunk for transfer
    pub snapshot_chunk_size: u64,

    /// Enable pre-vote extension (prevents disruptive elections)
    pub enable_prevote: bool,

    /// Enable leader lease (reduces read latency)
    pub enable_leader_lease: bool,
}

/// Network configuration for inter-node communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Connection timeout for peer connections
    pub connect_timeout: Duration,

    /// Request timeout for RPC calls
    pub request_timeout: Duration,

    /// Keep-alive interval for idle connections
    pub keep_alive_interval: Duration,

    /// Maximum message size in bytes
    pub max_message_size: usize,

    /// Number of concurrent connections per peer
    pub connections_per_peer: usize,

    /// Enable TLS for cluster communication
    pub enable_tls: bool,

    /// Path to TLS certificate
    pub tls_cert_path: Option<PathBuf>,

    /// Path to TLS private key
    pub tls_key_path: Option<PathBuf>,

    /// Path to CA certificate for peer verification
    pub tls_ca_path: Option<PathBuf>,
}

/// Sharding configuration for data distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingConfig {
    /// Number of virtual nodes in the hash ring
    pub virtual_nodes: u32,

    /// Replication factor for each shard
    pub replication_factor: u32,

    /// Minimum nodes required before enabling sharding
    pub min_nodes_for_sharding: u32,

    /// Enable automatic rebalancing when nodes join/leave
    pub auto_rebalance: bool,

    /// Delay before starting rebalance after membership change
    pub rebalance_delay: Duration,

    /// Maximum concurrent shard transfers
    pub max_concurrent_transfers: u32,
}

/// Service discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Discovery method
    pub method: DiscoveryMethod,

    /// Refresh interval for DNS discovery
    pub refresh_interval: Duration,

    /// DNS name for cluster discovery
    #[deprecated(since = "0.1.0", note = "Use dns.name instead")]
    pub dns_name: Option<String>,

    /// Static list of nodes (used when method is Static)
    pub static_nodes: Vec<String>,

    /// DNS service discovery configuration
    pub dns: Option<DnsConfig>,

    /// Consul service discovery configuration
    pub consul: Option<ConsulConfig>,

    /// etcd service discovery configuration
    pub etcd: Option<EtcdConfig>,
}

/// DNS service discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// DNS name for cluster discovery
    pub name: String,

    /// Default port to use when DNS doesn't provide port information (A/AAAA records)
    pub default_port: u16,
}

/// Consul service discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsulConfig {
    /// Consul address (e.g., "http://localhost:8500")
    pub address: String,

    /// Service name to query
    pub service_name: String,

    /// Optional datacenter
    pub datacenter: Option<String>,
}

/// etcd service discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtcdConfig {
    /// List of etcd endpoints (e.g., ["localhost:2379"])
    pub endpoints: Vec<String>,

    /// Prefix for node keys (e.g., "/neuroquantum/nodes/")
    pub prefix: String,
}

/// Method for discovering cluster nodes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DiscoveryMethod {
    /// Static list of nodes
    #[default]
    Static,

    /// DNS-based discovery (e.g., headless Kubernetes service)
    Dns,

    /// Consul service discovery
    Consul,

    /// etcd-based discovery
    Etcd,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            node_id: 1,
            bind_addr: "0.0.0.0:9000".parse().expect("valid default address"),
            advertise_addr: None,
            peers: Vec::new(),
            data_dir: PathBuf::from("./data/cluster"),
            raft: RaftConfig::default(),
            network: NetworkConfig::default(),
            sharding: ShardingConfig::default(),
            discovery: DiscoveryConfig::default(),
            manager: ClusterManagerConfig::default(),
        }
    }
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_millis(100),
            election_timeout_min: Duration::from_millis(300),
            election_timeout_max: Duration::from_millis(500),
            max_entries_per_rpc: 100,
            snapshot_threshold: 10_000,
            snapshot_chunk_size: 1024 * 1024, // 1 MB
            enable_prevote: true,
            enable_leader_lease: true,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
            keep_alive_interval: Duration::from_secs(30),
            max_message_size: 16 * 1024 * 1024, // 16 MB
            connections_per_peer: 2,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
        }
    }
}

impl Default for ShardingConfig {
    fn default() -> Self {
        Self {
            virtual_nodes: 150,
            replication_factor: 3,
            min_nodes_for_sharding: 3,
            auto_rebalance: true,
            rebalance_delay: Duration::from_secs(30),
            max_concurrent_transfers: 2,
        }
    }
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            method: DiscoveryMethod::Static,
            refresh_interval: Duration::from_secs(30),
            #[allow(deprecated)]
            dns_name: None,
            static_nodes: Vec::new(),
            dns: None,
            consul: None,
            etcd: None,
        }
    }
}

/// Builder for cluster configuration.
#[derive(Debug, Default)]
pub struct ClusterConfigBuilder {
    config: ClusterConfig,
}

impl ClusterConfig {
    /// Create a new builder for cluster configuration.
    #[must_use]
    pub fn builder() -> ClusterConfigBuilder {
        ClusterConfigBuilder::default()
    }

    /// Validate the configuration.
    pub fn validate(&self) -> ClusterResult<()> {
        if self.node_id == 0 {
            return Err(ClusterError::ConfigError(
                "Node ID must be greater than 0".into(),
            ));
        }

        if self.raft.election_timeout_min >= self.raft.election_timeout_max {
            return Err(ClusterError::ConfigError(
                "Election timeout min must be less than max".into(),
            ));
        }

        if self.raft.heartbeat_interval >= self.raft.election_timeout_min {
            return Err(ClusterError::ConfigError(
                "Heartbeat interval must be less than election timeout min".into(),
            ));
        }

        if self.sharding.replication_factor == 0 {
            return Err(ClusterError::ConfigError(
                "Replication factor must be greater than 0".into(),
            ));
        }

        if self.network.enable_tls {
            if self.network.tls_cert_path.is_none() {
                return Err(ClusterError::ConfigError(
                    "TLS enabled but no certificate path provided".into(),
                ));
            }
            if self.network.tls_key_path.is_none() {
                return Err(ClusterError::ConfigError(
                    "TLS enabled but no key path provided".into(),
                ));
            }
        }

        Ok(())
    }
}

impl ClusterConfigBuilder {
    /// Set the node ID.
    #[must_use]
    pub fn node_id(mut self, id: u64) -> Self {
        self.config.node_id = id;
        self
    }

    /// Set the bind address.
    #[must_use]
    pub fn bind_addr(mut self, addr: SocketAddr) -> Self {
        self.config.bind_addr = addr;
        self
    }

    /// Set the advertise address.
    #[must_use]
    pub fn advertise_addr(mut self, addr: SocketAddr) -> Self {
        self.config.advertise_addr = Some(addr);
        self
    }

    /// Set the peer addresses.
    #[must_use]
    pub fn peers(mut self, peers: Vec<String>) -> Self {
        self.config.peers = peers;
        self
    }

    /// Set the data directory.
    #[must_use]
    pub fn data_dir(mut self, path: PathBuf) -> Self {
        self.config.data_dir = path;
        self
    }

    /// Set the Raft configuration.
    #[must_use]
    pub fn raft(mut self, raft: RaftConfig) -> Self {
        self.config.raft = raft;
        self
    }

    /// Set the network configuration.
    #[must_use]
    pub fn network(mut self, network: NetworkConfig) -> Self {
        self.config.network = network;
        self
    }

    /// Set the sharding configuration.
    #[must_use]
    pub fn sharding(mut self, sharding: ShardingConfig) -> Self {
        self.config.sharding = sharding;
        self
    }

    /// Set the discovery configuration.
    #[must_use]
    pub fn discovery(mut self, discovery: DiscoveryConfig) -> Self {
        self.config.discovery = discovery;
        self
    }

    /// Set the cluster manager configuration.
    #[must_use]
    pub fn manager(mut self, manager: ClusterManagerConfig) -> Self {
        self.config.manager = manager;
        self
    }

    /// Set the heartbeat interval.
    #[must_use]
    pub fn heartbeat_interval(mut self, interval: Duration) -> Self {
        self.config.raft.heartbeat_interval = interval;
        self
    }

    /// Set the election timeout range.
    #[must_use]
    pub fn election_timeout(mut self, min: Duration, max: Duration) -> Self {
        self.config.raft.election_timeout_min = min;
        self.config.raft.election_timeout_max = max;
        self
    }

    /// Set the replication factor.
    #[must_use]
    pub fn replication_factor(mut self, factor: u32) -> Self {
        self.config.sharding.replication_factor = factor;
        self
    }

    /// Enable TLS with the given certificate and key paths.
    #[must_use]
    pub fn with_tls(mut self, cert_path: PathBuf, key_path: PathBuf, ca_path: PathBuf) -> Self {
        self.config.network.enable_tls = true;
        self.config.network.tls_cert_path = Some(cert_path);
        self.config.network.tls_key_path = Some(key_path);
        self.config.network.tls_ca_path = Some(ca_path);
        self
    }

    /// Build the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn build(self) -> ClusterResult<ClusterConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClusterConfig::default();
        assert_eq!(config.node_id, 1);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_builder() {
        let config = ClusterConfig::builder()
            .node_id(42)
            .bind_addr("127.0.0.1:9001".parse().unwrap())
            .peers(vec!["127.0.0.1:9002".into(), "127.0.0.1:9003".into()])
            .replication_factor(3)
            .build()
            .unwrap();

        assert_eq!(config.node_id, 42);
        assert_eq!(config.peers.len(), 2);
        assert_eq!(config.sharding.replication_factor, 3);
    }

    #[test]
    fn test_invalid_node_id() {
        let result = ClusterConfig::builder().node_id(0).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_election_timeout() {
        let result = ClusterConfig::builder()
            .election_timeout(Duration::from_millis(500), Duration::from_millis(300))
            .build();
        assert!(result.is_err());
    }
}
