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
    /// DNS configuration
    dns_config: Option<crate::config::DnsConfig>,
    /// Legacy DNS name (for backward compatibility)
    #[allow(deprecated)]
    dns_name: Option<String>,
    /// Consul configuration
    consul_config: Option<crate::config::ConsulConfig>,
    /// etcd configuration
    etcd_config: Option<crate::config::EtcdConfig>,
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
            dns_config: config.discovery.dns.clone(),
            #[allow(deprecated)]
            dns_name: config.discovery.dns_name.clone(),
            consul_config: config.discovery.consul.clone(),
            etcd_config: config.discovery.etcd.clone(),
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
        // Support both new dns config and legacy dns_name for backward compatibility
        let (dns_name, default_port) = if let Some(dns_config) = &self.dns_config {
            (dns_config.name.as_str(), dns_config.default_port)
        } else {
            #[allow(deprecated)]
            let name = self.dns_name.as_ref().ok_or_else(|| {
                ClusterError::ConfigError(
                    "DNS discovery enabled but no dns configuration provided".into(),
                )
            })?;
            (name.as_str(), 9000) // Default port for backward compatibility
        };

        debug!(dns_name, "Using DNS node discovery");

        use hickory_resolver::TokioAsyncResolver;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Create resolver from system configuration
        let resolver = TokioAsyncResolver::tokio_from_system_conf().map_err(|e| {
            ClusterError::DiscoveryError(format!("Failed to create DNS resolver: {}", e))
        })?;

        let mut peers = Vec::new();

        // Try SRV record lookup first (preferred for service discovery)
        match resolver.srv_lookup(dns_name).await {
            Ok(srv_records) => {
                debug!("Found {} SRV records", srv_records.iter().count());
                for srv in srv_records.iter() {
                    let target = srv.target().to_string();
                    let port = srv.port();

                    // Look up A/AAAA records for the target
                    match resolver.lookup_ip(target.as_str()).await {
                        Ok(ips) => {
                            for ip in ips.iter() {
                                let addr = SocketAddr::new(ip, port);

                                // Generate a deterministic node_id from the full address
                                let mut hasher = DefaultHasher::new();
                                addr.hash(&mut hasher);
                                let node_id = hasher.finish();

                                if node_id != self.local_node_id {
                                    peers.push(PeerInfo {
                                        node_id,
                                        addr,
                                        role: NodeRole::Follower,
                                        last_heartbeat: None,
                                        healthy: false,
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            warn!(target = %target, error = %e, "Failed to resolve target");
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback to A/AAAA record lookup
                debug!("No SRV records found, trying A/AAAA records");
                match resolver.lookup_ip(dns_name).await {
                    Ok(ips) => {
                        // Use configured default port for direct A/AAAA lookups
                        for ip in ips.iter() {
                            let addr = SocketAddr::new(ip, default_port);

                            // Generate a deterministic node_id from the full address
                            let mut hasher = DefaultHasher::new();
                            addr.hash(&mut hasher);
                            let node_id = hasher.finish();

                            if node_id != self.local_node_id {
                                peers.push(PeerInfo {
                                    node_id,
                                    addr,
                                    role: NodeRole::Follower,
                                    last_heartbeat: None,
                                    healthy: false,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        return Err(ClusterError::DiscoveryError(format!(
                            "Failed to resolve DNS name '{}': {}",
                            dns_name, e
                        )));
                    }
                }
            }
        }

        info!(count = peers.len(), "Discovered nodes via DNS");
        Ok(peers)
    }

    /// Discover nodes using Consul service discovery.
    async fn discover_consul(&self) -> ClusterResult<Vec<PeerInfo>> {
        let consul_config = self.consul_config.as_ref().ok_or_else(|| {
            ClusterError::ConfigError(
                "Consul discovery enabled but no consul configuration provided".into(),
            )
        })?;

        debug!(service = %consul_config.service_name, "Using Consul node discovery");

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Build Consul API URL with proper encoding
        let base_url = consul_config.address.trim_end_matches('/');
        let encoded_service = urlencoding::encode(&consul_config.service_name);

        let mut url = format!("{}/v1/catalog/service/{}", base_url, encoded_service);

        if let Some(dc) = &consul_config.datacenter {
            let encoded_dc = urlencoding::encode(dc);
            url.push_str(&format!("?dc={}", encoded_dc));
        }

        // Query Consul
        let client = reqwest::Client::new();
        let response =
            client.get(&url).send().await.map_err(|e| {
                ClusterError::DiscoveryError(format!("Failed to query Consul: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(ClusterError::DiscoveryError(format!(
                "Consul query failed with status: {}",
                response.status()
            )));
        }

        // Parse response
        let services: Vec<ConsulServiceEntry> = response.json().await.map_err(|e| {
            ClusterError::DiscoveryError(format!("Failed to parse Consul response: {}", e))
        })?;

        let mut peers = Vec::new();
        for service in services {
            // Parse address
            let addr: SocketAddr = format!("{}:{}", service.address, service.service_port)
                .parse()
                .map_err(|e| {
                    ClusterError::DiscoveryError(format!(
                        "Invalid address from Consul: {}:{} - {}",
                        service.address, service.service_port, e
                    ))
                })?;

            // Try to extract node ID from service tags or metadata
            let node_id = service
                .service_tags
                .iter()
                .find(|tag| tag.starts_with("node_id="))
                .and_then(|tag| tag.strip_prefix("node_id="))
                .and_then(|id| id.parse::<u64>().ok())
                .unwrap_or_else(|| {
                    // Generate deterministic node_id from address if not in tags
                    let mut hasher = DefaultHasher::new();
                    addr.hash(&mut hasher);
                    hasher.finish()
                });

            if node_id != self.local_node_id {
                peers.push(PeerInfo {
                    node_id,
                    addr,
                    role: NodeRole::Follower,
                    last_heartbeat: None,
                    healthy: false,
                });
            }
        }

        info!(count = peers.len(), "Discovered nodes via Consul");
        Ok(peers)
    }

    /// Discover nodes using etcd.
    async fn discover_etcd(&self) -> ClusterResult<Vec<PeerInfo>> {
        let etcd_config = self.etcd_config.as_ref().ok_or_else(|| {
            ClusterError::ConfigError(
                "etcd discovery enabled but no etcd configuration provided".into(),
            )
        })?;

        debug!(prefix = %etcd_config.prefix, "Using etcd node discovery");

        use etcd_client::{Client, GetOptions};

        // Connect to etcd cluster
        let mut client = Client::connect(&etcd_config.endpoints, None)
            .await
            .map_err(|e| {
                ClusterError::DiscoveryError(format!("Failed to connect to etcd: {}", e))
            })?;

        // Query for all keys with the given prefix
        let resp = client
            .get(
                etcd_config.prefix.as_bytes(),
                Some(GetOptions::new().with_prefix()),
            )
            .await
            .map_err(|e| ClusterError::DiscoveryError(format!("Failed to query etcd: {}", e)))?;

        let mut peers = Vec::new();
        for kv in resp.kvs() {
            // Parse the value as JSON containing node registration
            let value_str = std::str::from_utf8(kv.value()).map_err(|e| {
                ClusterError::DiscoveryError(format!("Invalid UTF-8 in etcd value: {}", e))
            })?;

            match serde_json::from_str::<NodeRegistration>(value_str) {
                Ok(registration) => {
                    // Skip self
                    if registration.node_id == self.local_node_id {
                        continue;
                    }

                    // Parse address
                    let addr: SocketAddr = registration.address.parse().map_err(|e| {
                        ClusterError::DiscoveryError(format!(
                            "Invalid address in etcd: {} - {}",
                            registration.address, e
                        ))
                    })?;

                    peers.push(PeerInfo {
                        node_id: registration.node_id,
                        addr,
                        role: NodeRole::Follower,
                        last_heartbeat: None,
                        healthy: false,
                    });
                }
                Err(e) => {
                    warn!(
                        key = ?std::str::from_utf8(kv.key()).unwrap_or("<invalid>"),
                        error = %e,
                        "Failed to parse node registration from etcd"
                    );
                }
            }
        }

        info!(count = peers.len(), "Discovered nodes via etcd");
        Ok(peers)
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

/// Consul service catalog entry (from Consul HTTP API).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ConsulServiceEntry {
    /// Service address
    address: String,
    /// Service port
    service_port: u16,
    /// Service tags
    #[serde(default)]
    service_tags: Vec<String>,
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

    #[tokio::test]
    async fn test_dns_discovery_missing_config() {
        let config = ClusterConfig {
            node_id: 1,
            discovery: crate::config::DiscoveryConfig {
                method: DiscoveryMethod::Dns,
                #[allow(deprecated)]
                dns_name: None, // Missing DNS name
                dns: None, // Missing new DNS config
                ..Default::default()
            },
            ..Default::default()
        };

        let service = DiscoveryService::new(&config).unwrap();
        let result = service.discover().await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClusterError::ConfigError(_)));
    }

    #[tokio::test]
    async fn test_consul_discovery_missing_config() {
        let config = ClusterConfig {
            node_id: 1,
            discovery: crate::config::DiscoveryConfig {
                method: DiscoveryMethod::Consul,
                consul: None, // Missing Consul config
                ..Default::default()
            },
            ..Default::default()
        };

        let service = DiscoveryService::new(&config).unwrap();
        let result = service.discover().await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClusterError::ConfigError(_)));
    }

    #[tokio::test]
    async fn test_etcd_discovery_missing_config() {
        let config = ClusterConfig {
            node_id: 1,
            discovery: crate::config::DiscoveryConfig {
                method: DiscoveryMethod::Etcd,
                etcd: None, // Missing etcd config
                ..Default::default()
            },
            ..Default::default()
        };

        let service = DiscoveryService::new(&config).unwrap();
        let result = service.discover().await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClusterError::ConfigError(_)));
    }

    #[test]
    fn test_consul_service_entry_deserialization() {
        let json = r#"{
            "Address": "127.0.0.1",
            "ServicePort": 9000,
            "ServiceTags": ["node_id=42", "primary"]
        }"#;

        let entry: ConsulServiceEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.address, "127.0.0.1");
        assert_eq!(entry.service_port, 9000);
        assert_eq!(entry.service_tags.len(), 2);
    }
}
