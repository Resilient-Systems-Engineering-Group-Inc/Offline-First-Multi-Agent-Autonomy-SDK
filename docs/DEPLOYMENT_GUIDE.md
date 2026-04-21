# Deployment Guide

Complete guide for deploying the Offline-First Multi-Agent Autonomy SDK.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Docker Deployment](#docker-deployment)
3. [Kubernetes Deployment](#kubernetes-deployment)
4. [Production Checklist](#production-checklist)
5. [Monitoring & Observability](#monitoring--observability)
6. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Prerequisites

- Docker 20.10+
- Docker Compose 2.0+
- Kubernetes 1.24+ (optional)
- 4GB RAM minimum
- 10GB disk space

### One-Command Deployment

```bash
# Clone and deploy
git clone https://github.com/your-org/Offline-First-Multi-Agent-Autonomy-SDK
cd Offline-First-Multi-Agent-Autonomy-SDK

# Start all services
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f dashboard
```

---

## Docker Deployment

### Local Development

```bash
# Build images
docker-compose build

# Start services
docker-compose up -d

# Access services
# Dashboard: http://localhost:3000
# Prometheus: http://localhost:9090
# Grafana: http://localhost:3001
```

### Production Docker

```bash
# Create production .env file
cat > .env << EOF
JWT_SECRET=$(openssl rand -hex 32)
DB_PASSWORD=$(openssl rand -hex 32)
GRAFANA_PASSWORD=$(openssl rand -hex 32)
EOF

# Build optimized images
docker-compose -f docker-compose.prod.yml build

# Deploy
docker-compose -f docker-compose.prod.yml up -d

# Health check
curl http://localhost:3000/api/health
```

### Docker Compose Services

| Service | Port | Description |
|---------|------|-------------|
| dashboard | 3000 | REST API + WebSocket |
| database | 5432 | PostgreSQL |
| prometheus | 9090 | Metrics collection |
| grafana | 3001 | Visualization |
| redis | 6379 | Caching (optional) |

### Custom Configuration

```yaml
# docker-compose.override.yml
services:
  dashboard:
    environment:
      - RUST_LOG=debug
      - DATABASE_URL=postgres://user:pass@database:5432/sdk_db
    ports:
      - "3000:3000"
      - "3001:3001"  # Additional metrics port
```

---

## Kubernetes Deployment

### Prerequisites

```bash
# Install kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
chmod +x kubectl
sudo mv kubectl /usr/local/bin/

# Install Helm
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
```

### Quick Deployment

```bash
# Create namespace
kubectl create namespace sdk

# Apply manifests
kubectl apply -f kubernetes/

# Check status
kubectl get all -n sdk

# View logs
kubectl logs -l app=sdk-dashboard -n sdk -f
```

### Helm Chart Deployment

```bash
# Add Helm repo
helm repo add sdk https://your-org.github.io/charts
helm repo update

# Install
helm install sdk sdk/sdk \
  --namespace sdk \
  --create-namespace \
  --values values-production.yaml
```

### Scaling

```bash
# Scale dashboard
kubectl scale deployment sdk-dashboard --replicas=5 -n sdk

# Horizontal Pod Autoscaler
kubectl autoscale deployment sdk-dashboard \
  --min=3 --max=10 --cpu-percent=80 -n sdk
```

### Resource Limits

```yaml
# values-production.yaml
resources:
  dashboard:
    requests:
      memory: "256Mi"
      cpu: "200m"
    limits:
      memory: "1Gi"
      cpu: "1000m"
  database:
    requests:
      memory: "512Mi"
      cpu: "500m"
    limits:
      memory: "2Gi"
      cpu: "2000m"
```

---

## Production Checklist

### Security

- [ ] Change all default passwords
- [ ] Generate strong JWT secrets
- [ ] Enable HTTPS/TLS
- [ ] Configure firewall rules
- [ ] Set up authentication
- [ ] Enable audit logging
- [ ] Rotate credentials regularly
- [ ] Use secrets management (Vault, K8s Secrets)

### Performance

- [ ] Configure connection pooling
- [ ] Enable database indexing
- [ ] Set up caching (Redis)
- [ ] Configure rate limiting
- [ ] Optimize database queries
- [ ] Enable compression
- [ ] Configure CDN for static assets

### Reliability

- [ ] Configure health checks
- [ ] Set up load balancing
- [ ] Enable automatic restarts
- [ ] Configure pod anti-affinity
- [ ] Set up backup strategy
- [ ] Configure disaster recovery
- [ ] Test failover procedures

### Monitoring

- [ ] Configure Prometheus scraping
- [ ] Set up Grafana dashboards
- [ ] Configure alerts
- [ ] Set up log aggregation
- [ ] Enable distributed tracing
- [ ] Configure capacity planning

### Compliance

- [ ] Enable audit trails
- [ ] Configure data retention
- [ ] Set up access controls
- [ ] Document security procedures
- [ ] Regular security audits
- [ ] Penetration testing

---

## Monitoring & Observability

### Prometheus Metrics

Access metrics at: `http://localhost:3000/metrics`

Key metrics:
- `sdk_tasks_completed_total` - Completed tasks
- `sdk_active_agents` - Active agent count
- `sdk_consensus_time_ms` - Consensus duration
- `sdk_message_latency_ms` - Message latency
- `sdk_cpu_usage_percent` - CPU usage
- `sdk_memory_usage_percent` - Memory usage

### Grafana Dashboards

Import dashboards from `monitoring/grafana/dashboards/`:

1. **System Overview** - Dashboard ID: 1001
2. **Task Metrics** - Dashboard ID: 1002
3. **Agent Performance** - Dashboard ID: 1003
4. **Workflow Analytics** - Dashboard ID: 1004

### Alerting Rules

```yaml
# prometheus-alerts.yml
groups:
  - name: sdk-alerts
    rules:
      - alert: HighTaskFailureRate
        expr: rate(sdk_tasks_failed_total[5m]) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High task failure rate detected"
          
      - alert: AgentDisconnected
        expr: sdk_active_agents < 1
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "All agents disconnected"
```

### Log Aggregation

```bash
# View logs with kubectl
kubectl logs -f deployment/sdk-dashboard -n sdk

# Export logs
kubectl logs deployment/sdk-dashboard -n sdk > logs.txt

# Search logs
kubectl logs -l app=sdk-dashboard -n sdk | grep "ERROR"
```

---

## Troubleshooting

### Common Issues

#### Dashboard not starting

```bash
# Check logs
docker-compose logs dashboard
kubectl logs -l app=sdk-dashboard -n sdk

# Check database connection
docker-compose exec database psql -U sdk_user -d sdk_db

# Verify environment variables
docker-compose exec dashboard env | grep DATABASE
```

#### High memory usage

```bash
# Check container memory
docker stats

# Increase limits
# Edit docker-compose.yml or kubernetes deployment
resources:
  limits:
    memory: "2Gi"  # Increase from 512Mi
```

#### Database connection errors

```bash
# Test database connection
docker-compose exec database pg_isready

# Check database logs
docker-compose logs database

# Reset database
docker-compose down -v
docker-compose up -d
```

#### Rate limiting issues

```bash
# Check rate limit config
curl -H "X-Client-ID: test" http://localhost:3000/api/health

# Increase limits in configuration
rate_limit:
  requests_per_minute: 200  # Increase from 100
```

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=debug
docker-compose up -d

# View detailed logs
docker-compose logs -f dashboard | grep DEBUG
```

### Performance Profiling

```bash
# Generate flamegraph
cargo install cargo-flamegraph
cargo flamegraph --bin dashboard

# Analyze database queries
docker-compose exec database \
  psql -U sdk_user -c "EXPLAIN ANALYZE SELECT * FROM tasks;"
```

### Backup & Restore

```bash
# Backup database
docker-compose exec database \
  pg_dump -U sdk_user sdk_db > backup.sql

# Restore database
docker-compose exec database \
  psql -U sdk_user sdk_db < backup.sql

# Backup volume
docker run --rm \
  -v sdk_dashboard-data:/data \
  -v $(pwd):/backup \
  alpine tar czf /backup/backup.tar.gz /data
```

---

## Environment Variables

### Dashboard

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | sqlite:sdk.db | Database connection string |
| `AUTH_JWT_SECRET` | (required) | JWT signing secret |
| `RUST_LOG` | info | Log level |
| `RATE_LIMIT_REQUESTS` | 100 | Requests per minute |

### Database

| Variable | Default | Description |
|----------|---------|-------------|
| `POSTGRES_DB` | sdk_db | Database name |
| `POSTGRES_USER` | sdk_user | Database user |
| `POSTGRES_PASSWORD` | (required) | Database password |

### Grafana

| Variable | Default | Description |
|----------|---------|-------------|
| `GF_SECURITY_ADMIN_PASSWORD` | admin | Admin password |

---

## Upgrade Guide

### From v0.x to v1.0

```bash
# 1. Backup data
docker-compose exec database pg_dump > backup.sql

# 2. Stop services
docker-compose down

# 3. Update images
docker-compose pull

# 4. Run migrations
docker-compose run --rm dashboard cargo run -- migrate

# 5. Start services
docker-compose up -d

# 6. Verify
curl http://localhost:3000/api/health
```

---

## Support

- **Documentation**: See `docs/` directory
- **GitHub Issues**: Report bugs and feature requests
- **Examples**: See `examples/` directory
- **Community**: Join our Slack/Discord

---

*Last Updated: 2026-03-27*
*Version: 1.0.0*
