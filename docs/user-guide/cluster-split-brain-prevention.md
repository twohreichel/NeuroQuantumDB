# Split Brain Prevention in Cluster Mode

## Overview

Split brain is a critical failure scenario in distributed systems where network partitions cause multiple nodes to believe they are the leader, potentially leading to data inconsistency and loss. NeuroQuantumDB implements comprehensive split brain prevention mechanisms to ensure data consistency and system reliability.

## What is Split Brain?

Split brain occurs when:
1. A network partition separates the cluster into isolated groups
2. Multiple groups independently elect their own leader
3. Both leaders accept write operations
4. Upon network recovery, conflicting data must be reconciled

This can lead to:
- Data inconsistency across nodes
- Data loss during reconciliation
- Service disruptions
- Undefined system state

## Prevention Mechanisms

### 1. Quorum-Based Writes

**How it works:**
- Before accepting any write operation, the leader verifies that a majority (quorum) of nodes are reachable
- Writes are rejected if quorum cannot be established
- Single-node clusters always have quorum

**Key Benefits:**
- Prevents split brain by ensuring only the partition with majority can accept writes
- Minority partitions automatically become read-only

**Configuration:**
```toml
[sharding]
replication_factor = 3  # Requires at least 2 nodes for quorum
```

**Quorum Calculation:**
- Quorum = (cluster_size / 2) + 1
- Example: 5-node cluster requires 3 nodes minimum
- Example: 3-node cluster requires 2 nodes minimum

### 2. Lease-Based Leadership

**How it works:**
- When promoted to leader, a node receives a time-limited lease
- Lease duration = 3 × heartbeat_interval (default: 300ms)
- The lease is renewed after each successful heartbeat to followers
- If the lease expires, the leader automatically steps down

**Key Benefits:**
- Prevents stale leaders from accepting writes after losing connectivity
- Ensures leadership is time-bounded
- Automatic failover when leader becomes isolated

**Configuration:**
```toml
[raft]
heartbeat_interval = "100ms"  # Lease = 3 × 100ms = 300ms
```

**Note:** Leader lease is always enabled in NeuroQuantumDB for split brain prevention. The lease duration is automatically calculated as 3× the heartbeat interval.

**Monitoring:**
Check leader lease status via cluster health API:
```bash
curl http://localhost:8080/api/v1/cluster/health
```

### 3. Fencing Tokens

**How it works:**
- Each write operation is tagged with a monotonically increasing fencing token
- Token consists of: (term, sequence_number)
- Storage layer validates tokens and rejects writes with stale tokens
- Prevents writes from old/isolated leaders

**Token Format:**
```rust
FencingToken {
    term: 5,        // Raft term number
    sequence: 142   // Monotonic sequence within term
}
```

**Key Benefits:**
- Prevents data corruption from stale writes
- Ensures write ordering even during leadership transitions
- Provides audit trail for write operations

**Token Validation:**
- Tokens must be from current or future term
- Within same term, sequence must be increasing
- Storage rejects any write with a stale token

### 4. Network Partition Detection

**How it works:**
- Continuous health checking of peer nodes
- Quorum status updated based on reachable peers
- Automatic detection of network partitions
- Leader steps down when minority is detected

**Detection Process:**
1. Health checks run every 5 seconds (configurable)
2. Peer marked unhealthy after missed heartbeats
3. Quorum status recalculated
4. Node enters read-only mode if in minority partition

**Configuration:**
```toml
[manager]
health_check_interval = "5s"
```

**Read-Only Mode:**
When a node loses quorum:
- State transitions to `ReadOnly`
- All write operations are rejected with `NoQuorum` error
- Read operations continue to work
- Automatic recovery when partition heals

## Operational Procedures

### Monitoring Split Brain Risks

**1. Check Cluster Health:**
```bash
# View cluster-wide health
curl http://localhost:8080/api/v1/cluster/health

# Expected output:
{
  "node_id": 1,
  "state": "Running",
  "role": "Leader",
  "leader_id": 1,
  "healthy_peers": 2,
  "total_peers": 2,
  "uptime_secs": 3600
}
```

**2. Monitor Quorum Status:**
- `healthy_peers` should be >= (total_peers / 2)
- `state` should be "Running" for normal operation
- `state`: "ReadOnly" indicates network partition

**3. Check Logs for Warnings:**
```bash
# Look for partition warnings
grep "Network partition detected" /var/log/neuroquantum/cluster.log
grep "Leader lost quorum" /var/log/neuroquantum/cluster.log
grep "Quorum status changed" /var/log/neuroquantum/cluster.log
```

### Responding to Network Partitions

**Scenario 1: Minor Partition Detected**
```
Node logs: "Network partition detected, entering read-only mode"
```

**Actions:**
1. Verify network connectivity between nodes
2. Check firewall rules and network configuration
3. Monitor for automatic recovery (partition healing)
4. No manual intervention needed if partition resolves quickly

**Scenario 2: Leader Step-Down**
```
Node logs: "Leader lost quorum, stepping down to follower"
```

**Actions:**
1. Verify that a new leader was elected in the majority partition
2. Check that the old leader transitioned to follower state
3. Confirm writes are being processed by the new leader
4. Monitor for partition healing and rejoin

**Scenario 3: Split Brain Detected (Critical)**
```
Error logs: "StaleToken" errors appearing
```

**Actions:**
1. **STOP ALL WRITES IMMEDIATELY**
2. Identify which partition has quorum
3. Shut down nodes in minority partition
4. Verify leader in majority partition
5. Fix network partition
6. Restart minority partition nodes (they will rejoin as followers)

### Chaos Testing

To verify split brain prevention, regularly test with simulated failures:

**Test 1: Network Partition Simulation**
```bash
# Isolate minority nodes using iptables
iptables -A INPUT -s <node2_ip> -j DROP
iptables -A OUTPUT -d <node2_ip> -j DROP

# Verify:
# 1. Leader steps down if in minority
# 2. Majority partition elects new leader
# 3. Writes succeed only in majority partition
# 4. Minority nodes enter read-only mode

# Restore network
iptables -D INPUT -s <node2_ip> -j DROP
iptables -D OUTPUT -d <node2_ip> -j DROP

# Verify automatic recovery
```

**Test 2: Leader Lease Expiry**
```bash
# Stop leader's network (using tc for delay)
tc qdisc add dev eth0 root netem delay 1000ms

# Verify:
# 1. Leader's lease expires
# 2. Leader steps down automatically
# 3. New leader elected
# 4. Old leader cannot accept writes

# Restore network
tc qdisc del dev eth0 root netem
```

**Test 3: Fencing Token Validation**
```bash
# This is tested automatically through the chaos tests
# Verify logs show rejected stale tokens after partition healing
grep "StaleToken" /var/log/neuroquantum/cluster.log
```

## Troubleshooting

### Issue: Writes Failing with "NoQuorum" Error

**Cause:** Node lost quorum (cannot reach majority of cluster)

**Solution:**
1. Check cluster status: `curl http://localhost:8080/api/v1/cluster/health`
2. Verify network connectivity to peer nodes
3. Ensure at least (n/2 + 1) nodes are healthy
4. Check for network partitions
5. Wait for partition to heal or restore failed nodes

### Issue: Node Stuck in "ReadOnly" State

**Cause:** Network partition not yet healed, or persistent connectivity issues

**Solution:**
1. Verify network connectivity restored
2. Check peer health status
3. Restart node if health checks not recovering
4. Check logs for specific connectivity errors

### Issue: "LeaseExpired" Errors on Leader

**Cause:** Leader unable to communicate with followers

**Solution:**
1. Check network latency between leader and followers
2. Verify heartbeat_interval is appropriate for network conditions
3. Increase heartbeat interval if network is slow:
   ```toml
   [raft]
   heartbeat_interval = "200ms"  # Increase if needed
   ```

### Issue: "StaleToken" Errors After Partition

**Cause:** Old leader attempting writes after being isolated

**Solution:**
- This is EXPECTED behavior and indicates split brain prevention is working
- Old leader should automatically step down
- Verify old leader is now a follower
- No action needed unless errors persist

## Best Practices

1. **Always use odd cluster sizes** (3, 5, 7) for clear quorum majority
2. **Monitor quorum status** continuously in production
3. **Test partition scenarios** regularly in staging environment
4. **Configure appropriate timeouts** based on network conditions
5. **Set up alerts** for:
   - Quorum loss warnings
   - Leader step-down events
   - Node transitions to ReadOnly state
6. **Document recovery procedures** for your specific deployment
7. **Keep time synchronized** across all nodes (use NTP)

## Configuration Reference

### Minimal Split Brain Prevention Config

```toml
[cluster]
node_id = 1
bind_addr = "0.0.0.0:9000"

[raft]
heartbeat_interval = "100ms"
election_timeout_min = "300ms"
election_timeout_max = "500ms"

[sharding]
replication_factor = 3  # Minimum 3 for production

[manager]
health_check_interval = "5s"
replication_timeout = "30s"
```

### Production-Recommended Config

```toml
[cluster]
node_id = 1
bind_addr = "0.0.0.0:9000"
peers = ["node2:9000", "node3:9000", "node4:9000", "node5:9000"]

[raft]
heartbeat_interval = "150ms"      # More lenient for production
election_timeout_min = "500ms"
election_timeout_max = "800ms"

[sharding]
replication_factor = 5            # 5-node cluster for high availability

[manager]
health_check_interval = "5s"
replication_timeout = "30s"
replication_cleanup_interval = "60s"
```

## Metrics and Monitoring

### Key Metrics to Track

1. **Quorum Status**
   - `neuroquantum_cluster_quorum_status{status="has_quorum|no_quorum"}`
   - Alert if `no_quorum` > 1 minute

2. **Leader Lease Validity**
   - `neuroquantum_cluster_leader_lease_valid{valid="true|false"}`
   - Alert if `valid=false` on leader

3. **Fencing Token Rejections**
   - `neuroquantum_cluster_stale_token_rejections_total`
   - Alert if > 0 (indicates potential split brain attempt)

4. **Node State Transitions**
   - `neuroquantum_cluster_node_state{state="running|readonly|error"}`
   - Alert on `readonly` or `error` states

5. **Healthy Peers Count**
   - `neuroquantum_cluster_healthy_peers`
   - Alert if < (cluster_size / 2)

## Additional Resources

- [Raft Consensus Algorithm](https://raft.github.io/)
- [Distributed Systems Patterns](https://martinfowler.com/articles/patterns-of-distributed-systems/)
- [Jepsen Testing for Distributed Systems](https://jepsen.io/)
- [Cluster Configuration Guide](configuration.md)
- [Monitoring Guide](monitoring.md)
- [Troubleshooting Guide](troubleshooting.md)
