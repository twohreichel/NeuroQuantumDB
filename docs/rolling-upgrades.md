# Rolling Upgrades Guide

This guide explains how to perform zero-downtime rolling upgrades of NeuroQuantumDB cluster nodes.

## Overview

Rolling upgrades allow you to upgrade cluster nodes one at a time without taking the entire cluster offline. The system ensures:

- **Zero downtime**: Cluster remains operational during upgrades
- **Protocol compatibility**: Version negotiation ensures nodes can communicate
- **Automatic failover**: Leader transitions are handled automatically
- **Health validation**: Nodes are validated before rejoining the cluster
- **Automatic rollback**: Failed upgrades can be rolled back automatically

## Protocol Versioning

NeuroQuantumDB uses protocol versioning to ensure compatibility between nodes during upgrades:

- **Protocol Version**: Current version of the node (e.g., v1, v2, v3)
- **Min Compatible Version**: Minimum version this node can communicate with

### Version Compatibility Rules

1. Nodes can communicate if their protocol versions are compatible
2. A node with version N can communicate with nodes running version >= MinCompatible
3. Upgrade nodes incrementally (don't skip versions)
4. Always upgrade all nodes in a cluster to the same version eventually

## Prerequisites

Before starting a rolling upgrade:

1. **Verify cluster health**:
   ```bash
   # Check that all nodes are healthy
   kubectl get pods -n neuroquantumdb
   ```

2. **Ensure minimum healthy nodes**:
   - Default minimum: 2 healthy nodes
   - For a 3-node cluster, at least 2 must be running at all times
   - For a 5-node cluster, at least 3 must be running (quorum = n/2 + 1)

3. **Back up cluster data** (recommended):
   ```bash
   # Create a backup before upgrade
   kubectl exec -n neuroquantumdb neuroquantumdb-0 -- /app/backup.sh
   ```

## Kubernetes Rolling Upgrade

### Automatic Rolling Update

For Kubernetes deployments, rolling updates are handled automatically:

```bash
# Update the image version
kubectl set image deployment/neuroquantumdb \
  neuroquantumdb=neuroquantumdb:v0.2.0 \
  -n neuroquantumdb

# Watch the rollout progress
kubectl rollout status deployment/neuroquantumdb -n neuroquantumdb
```

The deployment strategy is configured for zero-downtime upgrades:

```yaml
strategy:
  type: RollingUpdate
  rollingUpdate:
    maxSurge: 1          # Allow 1 extra pod during rollout
    maxUnavailable: 0    # Never take pods down before replacement is ready
```

### Manual Pod-by-Pod Upgrade

For more control, upgrade pods one at a time:

```bash
# 1. Cordon the node (optional, for node-level control)
kubectl cordon <node-name>

# 2. Update the pod
kubectl delete pod neuroquantumdb-0 -n neuroquantumdb

# 3. Wait for new pod to be ready
kubectl wait --for=condition=ready pod/neuroquantumdb-0 -n neuroquantumdb --timeout=120s

# 4. Verify health
kubectl exec -n neuroquantumdb neuroquantumdb-0 -- curl http://localhost:8080/health

# 5. Repeat for remaining pods
```

## Upgrade Process Details

Each node goes through the following phases during upgrade:

### 1. Draining Phase

- Node stops accepting new client connections
- Existing connections are drained (timeout: 30 seconds by default)
- If the node is the leader, leadership is transferred to a follower

### 2. Shutdown Phase

- Raft consensus module stops gracefully
- Network connections are closed
- Node state is persisted

### 3. Upgrade Phase

- Container/binary is replaced with new version
- Configuration is updated if needed
- Node is restarted with new version

### 4. Rejoin Phase

- Node performs protocol version handshake with peers
- Health checks are executed
- Node rejoins the Raft cluster
- Once healthy, node begins accepting traffic

### 5. Validation Phase

- Verify the node is in "Running" state
- Check that it can reach quorum
- Ensure protocol compatibility with peers

## Configuration

### Cluster Configuration

Configure upgrade behavior in your cluster config:

```toml
[cluster.manager.upgrades]
drain_timeout_secs = 30              # Time to wait for connections to drain
health_check_interval_secs = 5       # Interval between health checks
min_healthy_nodes = 2                # Minimum healthy nodes during upgrade
rollback_on_failure = true           # Enable automatic rollback
protocol_version = 1                 # Current protocol version
min_compatible_version = 1           # Minimum compatible version
```

### Environment Variables

For Kubernetes deployments:

```yaml
env:
  - name: CLUSTER_DRAIN_TIMEOUT_SECS
    value: "30"
  - name: CLUSTER_HEALTH_CHECK_INTERVAL_SECS
    value: "5"
  - name: CLUSTER_MIN_HEALTHY_NODES
    value: "2"
  - name: CLUSTER_ROLLBACK_ON_FAILURE
    value: "true"
  - name: CLUSTER_PROTOCOL_VERSION
    value: "1"
  - name: CLUSTER_MIN_COMPATIBLE_VERSION
    value: "1"
```

## Canary Deployments

For high-risk upgrades, use canary deployments to test on a single node first:

```bash
# 1. Update a single pod with new version
kubectl set image pod/neuroquantumdb-0 \
  neuroquantumdb=neuroquantumdb:v0.2.0 \
  -n neuroquantumdb

# 2. Monitor the canary node for issues
kubectl logs -f neuroquantumdb-0 -n neuroquantumdb

# 3. Check metrics
kubectl exec -n neuroquantumdb neuroquantumdb-0 -- curl http://localhost:8080/metrics

# 4. If canary is healthy, proceed with remaining nodes
kubectl set image deployment/neuroquantumdb \
  neuroquantumdb=neuroquantumdb:v0.2.0 \
  -n neuroquantumdb --record
```

## Rollback Procedure

### Automatic Rollback

If `rollback_on_failure = true`, the system will automatically roll back on upgrade failure.

### Manual Rollback

If you need to manually roll back:

```bash
# Rollback to previous deployment
kubectl rollout undo deployment/neuroquantumdb -n neuroquantumdb

# Check rollback status
kubectl rollout status deployment/neuroquantumdb -n neuroquantumdb

# Verify all pods are healthy
kubectl get pods -n neuroquantumdb
```

## Monitoring Upgrades

### Check Upgrade Progress

```bash
# View rollout status
kubectl rollout status deployment/neuroquantumdb -n neuroquantumdb

# Check pod status
kubectl get pods -n neuroquantumdb -w

# View events
kubectl get events -n neuroquantumdb --sort-by='.lastTimestamp'
```

### Check Cluster Health

```bash
# Check cluster health endpoint
kubectl exec -n neuroquantumdb neuroquantumdb-0 -- \
  curl -s http://localhost:8080/health | jq

# Check Raft status
kubectl exec -n neuroquantumdb neuroquantumdb-0 -- \
  curl -s http://localhost:8080/cluster/status | jq
```

### View Logs

```bash
# View logs for upgrade events
kubectl logs -f deployment/neuroquantumdb -n neuroquantumdb | \
  grep -E "upgrade|draining|health_check"
```

## Troubleshooting

### Upgrade Stuck

If the upgrade appears stuck:

```bash
# Check pod status
kubectl describe pod <pod-name> -n neuroquantumdb

# Check events
kubectl get events -n neuroquantumdb

# Force delete stuck pod (last resort)
kubectl delete pod <pod-name> --force --grace-period=0 -n neuroquantumdb
```

### Health Check Failures

If health checks fail after upgrade:

```bash
# Check logs
kubectl logs <pod-name> -n neuroquantumdb --tail=100

# Check if node can reach peers
kubectl exec -n neuroquantumdb <pod-name> -- \
  curl -v http://<peer-pod>:9000/health

# Verify protocol version compatibility
kubectl exec -n neuroquantumdb <pod-name> -- \
  curl http://localhost:8080/cluster/protocol-version
```

### Quorum Lost

If quorum is lost during upgrade:

1. **Stop the upgrade immediately**:
   ```bash
   kubectl rollout pause deployment/neuroquantumdb -n neuroquantumdb
   ```

2. **Check how many nodes are healthy**:
   ```bash
   kubectl get pods -n neuroquantumdb
   ```

3. **Bring nodes back online** or **roll back**:
   ```bash
   kubectl rollout undo deployment/neuroquantumdb -n neuroquantumdb
   ```

4. **Resume once quorum is restored**:
   ```bash
   kubectl rollout resume deployment/neuroquantumdb -n neuroquantumdb
   ```

## Best Practices

1. **Test upgrades in staging** before production
2. **Monitor metrics** during upgrades (latency, error rates)
3. **Schedule upgrades** during low-traffic periods
4. **Keep nodes at similar versions** (don't let versions drift)
5. **Document protocol version changes** in release notes
6. **Maintain backward compatibility** for at least one version
7. **Use PodDisruptionBudgets** to enforce minimum availability

### Example PodDisruptionBudget

```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: neuroquantumdb-pdb
  namespace: neuroquantumdb
spec:
  minAvailable: 2  # Always keep at least 2 pods running
  selector:
    matchLabels:
      app.kubernetes.io/name: neuroquantumdb
```

## Version Upgrade Matrix

| From Version | To Version | Compatible | Notes |
|--------------|------------|------------|-------|
| v1           | v2         | ✅ Yes     | Direct upgrade supported |
| v1           | v3         | ❌ No      | Must upgrade to v2 first |
| v2           | v3         | ✅ Yes     | Direct upgrade supported |

## Support

For issues or questions:

- GitHub Issues: https://github.com/neuroquantumdb/neuroquantumdb/issues
- Documentation: https://docs.neuroquantumdb.org
- Community Forum: https://community.neuroquantumdb.org
