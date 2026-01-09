# NeuroQuantumDB Cluster E2E Tests

This directory contains end-to-end tests for the NeuroQuantumDB cluster functionality.

## Overview

The cluster E2E tests validate distributed consensus, fault tolerance, and data consistency across multi-node deployments. These tests are designed to:

- Verify cluster formation with 3/5/7 nodes
- Test leader election and failover scenarios
- Validate data consistency across nodes
- Simulate network partitions and split-brain recovery
- Perform chaos engineering tests

## Test Categories

### 1. Basic Cluster Operations
- Cluster formation with 3, 5, and 7 nodes
- Leader election on startup
- Client routing to leader
- Read/write operations across cluster

### 2. Failure Scenarios
- Leader node failure and re-election
- Follower node failure and recovery
- Network partition (split-brain)
- Multiple simultaneous failures
- Loss of quorum handling

### 3. Data Consistency Tests
- Write to leader, read from follower
- Consistency after leader failover
- Stale read detection
- Linearizability under concurrent writes

### 4. Performance Under Failure
- Latency during leader election
- Recovery time objectives (RTO)
- Throughput during failover

### 5. Chaos Engineering
- Random node kills
- Network delay injection
- Split-brain recovery
- Concurrent load with failures

## Running the Tests

### Unit-style Tests (Simulated Cluster)

The simulated tests run quickly and don't require Docker:

```bash
# Run all non-ignored cluster E2E tests
cargo test --package neuroquantum-core --test cluster_e2e_tests

# Run with verbose output
cargo test --package neuroquantum-core --test cluster_e2e_tests -- --nocapture

# Run long-running tests (marked with #[ignore])
cargo test --package neuroquantum-core --test cluster_e2e_tests -- --ignored --nocapture

# Run all tests including ignored
cargo test --package neuroquantum-core --test cluster_e2e_tests -- --include-ignored --nocapture
```

### Docker-based Integration Tests

For full integration testing with real network isolation:

```bash
# Navigate to cluster-test directory
cd docker/cluster-test

# Start 3-node cluster
docker-compose -f docker-compose.cluster-test.yml up -d

# Wait for cluster to form (check health)
docker-compose -f docker-compose.cluster-test.yml ps

# Start 5-node cluster
docker-compose -f docker-compose.cluster-test.yml --profile five-node up -d

# Start 7-node cluster
docker-compose -f docker-compose.cluster-test.yml --profile seven-node up -d

# Enable chaos engineering tools
docker-compose -f docker-compose.cluster-test.yml --profile chaos up -d

# Enable monitoring
docker-compose -f docker-compose.cluster-test.yml --profile monitoring up -d

# Stop cluster and clean up
docker-compose -f docker-compose.cluster-test.yml down -v
```

### Chaos Engineering with Toxiproxy

```bash
# Start cluster with chaos tools
docker-compose -f docker-compose.cluster-test.yml --profile chaos up -d

# Create proxy for node1
curl -X POST http://localhost:8474/proxies -d '{
  "name": "node1_proxy",
  "listen": "[::]:19001",
  "upstream": "node1:9000"
}'

# Add latency toxic
curl -X POST http://localhost:8474/proxies/node1_proxy/toxics -d '{
  "name": "latency",
  "type": "latency",
  "attributes": {"latency": 100, "jitter": 50}
}'

# Simulate network partition (disable proxy)
curl -X POST http://localhost:8474/proxies/node1_proxy -d '{"enabled": false}'

# Re-enable proxy (heal partition)
curl -X POST http://localhost:8474/proxies/node1_proxy -d '{"enabled": true}'
```

## Test Configuration

Key configuration parameters can be adjusted in the test file:

```rust
mod config {
    /// Default timeout for cluster formation
    pub const CLUSTER_FORMATION_TIMEOUT: Duration = Duration::from_secs(10);

    /// Default timeout for leader election
    pub const LEADER_ELECTION_TIMEOUT: Duration = Duration::from_secs(5);

    /// Number of chaos cycles to run
    pub const CHAOS_CYCLES: usize = 10;

    /// Number of concurrent writers during chaos test
    pub const CONCURRENT_WRITERS: usize = 4;
}
```

## Expected Results

All tests should pass with the following acceptance criteria:

- [x] 3-node cluster E2E tests pass
- [x] 5-node cluster E2E tests pass
- [x] Leader failover test works
- [x] Network partition test works
- [x] Split-brain recovery test works
- [x] Data consistency verified after failures
- [ ] Tests run in CI pipeline
- [x] Documentation for running cluster tests

## Troubleshooting

### Tests failing with port conflicts

```bash
# The tests use dynamic port allocation starting from 40000
# If you see port conflicts, check for other processes using these ports
lsof -i :40000-40100
```

### Docker containers not starting

```bash
# Check container logs
docker-compose -f docker-compose.cluster-test.yml logs node1

# Verify network connectivity
docker exec neuroquantumdb-node1 ping node2
```

### Chaos tests causing issues

```bash
# Stop all chaos tools
docker-compose -f docker-compose.cluster-test.yml --profile chaos down

# Reset Toxiproxy
curl -X POST http://localhost:8474/reset
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Cluster E2E Test Suite                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │   Node 1    │    │   Node 2    │    │   Node 3    │     │
│  │  (Leader)   │◄──►│  (Follower) │◄──►│  (Follower) │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│         │                  │                  │             │
│         └──────────────────┼──────────────────┘             │
│                            │                                │
│                    ┌───────▼───────┐                        │
│                    │  Raft Log     │                        │
│                    │  Replication  │                        │
│                    └───────────────┘                        │
│                                                             │
│  Optional Components:                                       │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │  Toxiproxy  │    │   Pumba     │    │ Prometheus  │     │
│  │ (Network    │    │ (Container  │    │ (Metrics)   │     │
│  │  Chaos)     │    │  Chaos)     │    │             │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Related Files

- `crates/neuroquantum-core/tests/cluster_e2e_tests.rs` - Main test file
- `crates/neuroquantum-cluster/src/` - Cluster implementation
- `docker/cluster-test/docker-compose.cluster-test.yml` - Docker setup
- `crates/neuroquantum-core/tests/chaos_engineering_tests.rs` - Chaos tests for storage

## Contributing

When adding new cluster E2E tests:

1. Follow the existing test structure and naming conventions
2. Use `#[ignore]` for long-running tests
3. Update the stats reporting for new test categories
4. Document new chaos scenarios
5. Ensure tests clean up resources properly
