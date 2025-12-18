//! # NeuroQuantumDB Cluster Module
//!
//! This crate provides distributed cluster management for NeuroQuantumDB,
//! enabling horizontal scaling through multi-node deployments.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    NeuroQuantumDB Cluster                    │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
//! │  │   Node 1    │    │   Node 2    │    │   Node 3    │     │
//! │  │  (Leader)   │◄──►│  (Follower) │◄──►│  (Follower) │     │
//! │  └─────────────┘    └─────────────┘    └─────────────┘     │
//! │         │                  │                  │             │
//! │         └──────────────────┼──────────────────┘             │
//! │                            │                                │
//! │                    ┌───────▼───────┐                        │
//! │                    │  Raft Log     │                        │
//! │                    │  Replication  │                        │
//! │                    └───────────────┘                        │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Core Components
//!
//! - **Raft Consensus**: Leader election and log replication using `openraft`
//! - **gRPC Transport**: Inter-node communication via `tonic`
//! - **Consistent Hashing**: Data sharding across nodes
//! - **Service Discovery**: DNS-based or static node discovery
//!
//! ## Usage
//!
//! ```rust,no_run
//! use neuroquantum_cluster::{ClusterConfig, ClusterNode};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ClusterConfig::builder()
//!         .node_id(1)
//!         .bind_addr("0.0.0.0:9000".parse()?)
//!         .peers(vec!["node2:9000".into(), "node3:9000".into()])
//!         .build()?;
//!
//!     let node = ClusterNode::new(config).await?;
//!     node.start().await?;
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod consensus;
pub mod discovery;
pub mod error;
pub mod network;
pub mod node;
pub mod replication;
pub mod sharding;

// Re-export main types
pub use config::ClusterConfig;
pub use error::{ClusterError, ClusterResult};
pub use node::{ClusterNode, NodeId, NodeRole, NodeState};
pub use sharding::ShardManager;
