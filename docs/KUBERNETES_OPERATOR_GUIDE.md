# Kubernetes Operator Guide

## Overview

The SDK Kubernetes Operator automates deployment, scaling, and management of the Multi-Agent SDK in Kubernetes clusters.

## Features

- ✅ **Automatic Deployment** - Deploy agents, tasks, and workflows via CRDs
- ✅ **Self-Healing** - Automatically restart failed pods
- ✅ **Scaling** - Horizontal pod autoscaling
- ✅ **Configuration Management** - Centralized cluster configuration
- ✅ **Service Discovery** - Automatic service creation
- ✅ **Rolling Updates** - Zero-downtime deployments

## Installation

### Prerequisites

- Kubernetes 1.24+
- kubectl
- Helm 3.0+

### Quick Start

```bash
# Install operator
helm install sdk-operator ./kubernetes/operator-chart \
  --namespace sdk-system \
  --create-namespace

# Verify installation
kubectl get pods -n sdk-system
```

## Custom Resource Definitions

### Agent CRD

Deploy agents with custom specifications:

```yaml
apiVersion: sdk.autonomy.io/v1alpha1
kind: Agent
metadata:
  name: exploration-agent
  namespace: default
spec:
  name: exploration-agent
  image: sdk-agent:latest
  replicas: 3
  capabilities:
    - navigation
    - lidar
    - mapping
  resources:
    cpu_limit: "2"
    memory_limit: "4Gi"
    cpu_request: "500m"
    memory_request: "1Gi"
  config:
    mesh_enabled: true
    tracing_enabled: true
    metrics_enabled: true
    log_level: info
```

Apply the configuration:

```bash
kubectl apply -f agent.yaml
```

### Task CRD

Define tasks declaratively:

```yaml
apiVersion: sdk.autonomy.io/v1alpha1
kind: Task
metadata:
  name: zone-exploration
  namespace: default
spec:
  description: Explore warehouse zone A
  priority: 150
  requiredCapabilities:
    - navigation
    - lidar
  parameters:
    zone: "A"
    duration: 300
```

### Workflow CRD

Define workflows with multiple tasks:

```yaml
apiVersion: sdk.autonomy.io/v1alpha1
kind:Workflow
metadata:
  name: warehouse-workflow
  namespace: default
spec:
  name: warehouse-workflow
  version: "1.0.0"
  description: Warehouse automation workflow
  tasks:
    - id: scan
      name: Scan Area
      action: lidar-scan
      parameters:
        resolution: 0.1
      retries: 3
    - id: map
      name: Create Map
      action: build-map
      parameters:
        algorithm: octomap
      retries: 2
  triggers:
    - event: task-completed
      conditions:
        task: scan
        status: success
```

### ClusterConfig CRD

Configure the entire cluster:

```yaml
apiVersion: sdk.autonomy.io/v1alpha1
kind: ClusterConfig
metadata:
  name: cluster-config
  namespace: default
spec:
  mesh_config:
    protocol: libp2p
    discovery_interval_ms: 5000
    heartbeat_interval_ms: 1000
    max_peers: 50
  security_config:
    enable_pq_crypto: true
    jwt_expiry_secs: 3600
    rbac_enabled: true
  monitoring_config:
    prometheus_enabled: true
    jaeger_enabled: true
    metrics_port: 9090
    tracing_sample_rate: 1.0
  edge_config:
    enable_edge_computing: true
    sync_interval_ms: 5000
    max_edge_tasks: 100
```

## Operator Commands

### List Resources

```bash
# List all agents
kubectl get agents

# List all tasks
kubectl get tasks

# List all workflows
kubectl get workflows

# Get detailed info
kubectl get agent exploration-agent -o yaml
```

### Scale Agents

```bash
# Scale agent replicas
kubectl scale agent exploration-agent --replicas=5
```

### View Logs

```bash
# View agent logs
kubectl logs -f deployment/exploration-agent-deployment
```

### Troubleshoot

```bash
# Check operator status
kubectl get pods -n sdk-system

# Check operator logs
kubectl logs -f deployment/sdk-operator -n sdk-system

# Describe resource
kubectl describe agent exploration-agent
```

## Advanced Features

### Horizontal Pod Autoscaling

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: agent-hpa
  namespace: default
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: exploration-agent-deployment
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

### Rolling Updates

```bash
# Update agent image
kubectl set image deployment/exploration-agent-deployment \
  sdk-agent=sdk-agent:v1.1.0

# Monitor rollout
kubectl rollout status deployment/exploration-agent-deployment

# Rollback if needed
kubectl rollout undo deployment/exploration-agent-deployment
```

### Resource Quotas

```yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: sdk-quota
  namespace: default
spec:
  hard:
    requests.cpu: "10"
    requests.memory: "20Gi"
    limits.cpu: "20"
    limits.memory: "40Gi"
    pods: "50"
```

## Monitoring

### Prometheus Metrics

The operator exposes metrics at `/metrics`:

- `sdk_operator_reconciliations_total` - Total reconciliations
- `sdk_operator_reconciliations_failed_total` - Failed reconciliations
- `sdk_operator_resources` - Current resource count
- `sdk_operator_sync_duration_seconds` - Sync duration

### Grafana Dashboard

Import the provided dashboard:

```bash
kubectl apply -f kubernetes/grafana-dashboard.yaml
```

## Security

### RBAC

The operator uses Kubernetes RBAC:

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: sdk-operator
rules:
  - apiGroups: ["sdk.autonomy.io"]
    resources: ["agents", "tasks", "workflows"]
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
  - apiGroups: ["apps"]
    resources: ["deployments"]
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
```

### Network Policies

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: agent-network-policy
  namespace: default
spec:
  podSelector:
    matchLabels:
      app: agent
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: sdk-system
  egress:
    - to:
        - namespaceSelector:
            matchLabels:
              name: monitoring
```

## Examples

### Complete Deployment

```bash
# Apply all resources
kubectl apply -f kubernetes/examples/complete-deployment/

# Verify
kubectl get all -n default
```

### Edge Computing

```yaml
apiVersion: sdk.autonomy.io/v1alpha1
kind: ClusterConfig
metadata:
  name: edge-config
spec:
  edge_config:
    enable_edge_computing: true
    sync_interval_ms: 3000
    max_edge_tasks: 50
```

## Troubleshooting

### Common Issues

**Operator not reconciling:**
```bash
kubectl logs -f deployment/sdk-operator -n sdk-system
```

**Agent pods not starting:**
```bash
kubectl describe pod agent-pod-name
kubectl logs pod agent-pod-name
```

**Resource quota exceeded:**
```bash
kubectl describe resourcequota sdk-quota
```

### Debug Mode

```bash
# Enable debug logging
kubectl set env deployment/sdk-operator -n sdk-system RUST_LOG=debug
```

## Best Practices

1. **Use Namespaces** - Separate environments with namespaces
2. **Resource Limits** - Always set CPU/memory limits
3. **Health Checks** - Configure liveness/readiness probes
4. **Monitoring** - Enable Prometheus metrics
5. **Backup** - Regularly backup CRD configurations
6. **Rolling Updates** - Use rolling updates for zero-downtime

## Next Steps

- [Kubernetes Operator API Reference](./OPERATOR_API_REFERENCE.md)
- [Custom Resource Definitions](./CRD_REFERENCE.md)
- [Deployment Examples](./DEPLOYMENT_EXAMPLES.md)

---

*Last Updated: 2026-03-27*
