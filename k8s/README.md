# NeuroQuantumDB Kubernetes Deployment

This directory contains Kubernetes manifests for deploying NeuroQuantumDB in a production environment.

> **⚠️ Important Note:** This documentation describes Kubernetes deployment with multiple replicas (horizontal scaling of the application), **NOT** the NeuroQuantumDB cluster module. Each replica runs as an independent single-node NeuroQuantumDB instance. The NeuroQuantumDB cluster module (multi-node database cluster with Raft consensus) is currently in Beta and not production-ready. See the [main README](../README.md#-cluster-mode-beta) for details.

## Prerequisites

- Kubernetes cluster (v1.25+)
- `kubectl` configured with cluster access
- NGINX Ingress Controller (for external access)
- cert-manager (optional, for automatic TLS certificates)
- StorageClass configured (for PersistentVolumes)

## Quick Start

### 1. Review and Update Secrets

**IMPORTANT**: Before deploying, update the secrets in `secret.yaml`:

```bash
# Generate secure JWT secret
openssl rand -base64 48

# Generate secure API key
openssl rand -hex 32
```

### 2. Deploy with Kustomize

```bash
# Apply all manifests
kubectl apply -k k8s/

# Or apply individually
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secret.yaml
kubectl apply -f k8s/pvc.yaml
kubectl apply -f k8s/serviceaccount.yaml
kubectl apply -f k8s/redis.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
kubectl apply -f k8s/ingress.yaml
kubectl apply -f k8s/hpa.yaml
kubectl apply -f k8s/pdb.yaml
kubectl apply -f k8s/networkpolicy.yaml
kubectl apply -f k8s/prometheus.yaml
```

### 3. Verify Deployment

```bash
# Check namespace
kubectl get all -n neuroquantumdb

# Check pod status
kubectl get pods -n neuroquantumdb -w

# View logs
kubectl logs -n neuroquantumdb -l app.kubernetes.io/name=neuroquantumdb -f

# Check services
kubectl get svc -n neuroquantumdb
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Kubernetes Cluster                       │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                  neuroquantumdb namespace              │  │
│  │                                                        │  │
│  │   ┌─────────┐    ┌──────────────────────────────┐     │  │
│  │   │ Ingress │───▶│   NeuroQuantumDB Deployment  │     │  │
│  │   │ (nginx) │    │   (2-10 replicas with HPA)   │     │  │
│  │   └─────────┘    └──────────────┬───────────────┘     │  │
│  │                                 │                      │  │
│  │                                 ▼                      │  │
│  │                          ┌───────────┐                 │  │
│  │                          │   Redis   │                 │  │
│  │                          │  (cache)  │                 │  │
│  │                          └───────────┘                 │  │
│  │                                                        │  │
│  │   ┌────────────┐    ┌─────────────────────────────┐   │  │
│  │   │ Prometheus │◀───│  Metrics Scraping (15s)     │   │  │
│  │   └────────────┘    └─────────────────────────────┘   │  │
│  │                                                        │  │
│  │   Storage: PVCs for data, logs, redis, prometheus     │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Components

| Component | Purpose | Manifest |
|-----------|---------|----------|
| Namespace | Resource isolation | `namespace.yaml` |
| ConfigMap | Application configuration | `configmap.yaml` |
| Secret | Sensitive data (JWT, API keys) | `secret.yaml` |
| PVC | Persistent storage | `pvc.yaml` |
| ServiceAccount | RBAC identity | `serviceaccount.yaml` |
| Deployment | Main application | `deployment.yaml` |
| Redis | Rate limiting & caching | `redis.yaml` |
| Service | Internal networking | `service.yaml` |
| Ingress | External access with TLS | `ingress.yaml` |
| HPA | Auto-scaling | `hpa.yaml` |
| PDB | Availability guarantee | `pdb.yaml` |
| NetworkPolicy | Network security | `networkpolicy.yaml` |
| Prometheus | Metrics collection | `prometheus.yaml` |

## Configuration

### Scaling

Edit `hpa.yaml` to adjust auto-scaling parameters:

```yaml
spec:
  minReplicas: 2      # Minimum pods
  maxReplicas: 10     # Maximum pods
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          averageUtilization: 70  # Scale up at 70% CPU
```

### Resources

Edit `deployment.yaml` to adjust resource limits:

```yaml
resources:
  requests:
    memory: "256Mi"
    cpu: "250m"
  limits:
    memory: "512Mi"
    cpu: "1000m"
```

### Storage

Edit `pvc.yaml` to adjust storage sizes and classes:

```yaml
spec:
  resources:
    requests:
      storage: 10Gi
  storageClassName: your-storage-class  # Uncomment and set
```

## Security Features

- **Pod Security**: Non-root user, read-only filesystem, dropped capabilities
- **Network Policies**: Restrict traffic between pods
- **RBAC**: Minimal permissions with ServiceAccount
- **Secrets Management**: Kubernetes Secrets (consider Vault for production)
- **TLS**: Automatic certificate management with cert-manager

## Monitoring

Access Prometheus:

```bash
kubectl port-forward -n neuroquantumdb svc/prometheus 9090:9090
```

Then open: http://localhost:9090

### Key Metrics

- `neuroquantumdb_http_requests_total`
- `neuroquantumdb_query_duration_seconds`
- `neuroquantumdb_active_connections`
- `neuroquantumdb_synaptic_learning_cycles_total`

## Troubleshooting

### Pod Not Starting

```bash
# Check events
kubectl describe pod -n neuroquantumdb -l app.kubernetes.io/name=neuroquantumdb

# Check logs
kubectl logs -n neuroquantumdb -l app.kubernetes.io/name=neuroquantumdb --previous
```

### Storage Issues

```bash
# Check PVC status
kubectl get pvc -n neuroquantumdb

# Check PV binding
kubectl describe pvc -n neuroquantumdb neuroquantumdb-data
```

### Network Issues

```bash
# Test connectivity from pod
kubectl exec -n neuroquantumdb -it deploy/neuroquantumdb -- /bin/sh -c "wget -q -O- http://redis:6379/ping"

# Check network policies
kubectl describe networkpolicy -n neuroquantumdb
```

## Production Checklist

- [ ] Update secrets with secure values
- [ ] Configure appropriate StorageClass
- [ ] Set up external DNS for Ingress
- [ ] Configure cert-manager for TLS
- [ ] Set up log aggregation (ELK, Loki, etc.)
- [ ] Configure alerting in Prometheus
- [ ] Set up backup for PVCs
- [ ] Test disaster recovery procedures
- [ ] Configure resource quotas for namespace
- [ ] Set up pod security policies/standards
