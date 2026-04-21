# Dashboard

Web dashboard for monitoring and controlling the Offline-First Multi-Agent Autonomy SDK.

## Features

### Backend (Rust)
- **REST API** - Full CRUD operations for agents, tasks, and workflows
- **WebSocket** - Real-time updates for all system events
- **Prometheus Metrics** - Comprehensive monitoring and alerting
- **Async-first** - High-performance async/await based implementation

### Frontend (Yew/WASM)
- **Real-time UI** - Live updates via WebSocket
- **Agent Visualization** - Network topology and status
- **Task Management** - Create, assign, monitor tasks
- **Workflow Control** - Start, pause, resume workflows
- **Metrics Dashboard** - Charts and graphs for performance monitoring

## API Endpoints

### Health
```
GET /api/health
```
Returns system health status.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "timestamp": 1234567890,
  "agents_count": 5,
  "tasks_count": 10,
  "workflows_count": 2
}
```

### Metrics
```
GET /api/metrics
```
Returns system metrics summary.

**Response:**
```json
{
  "timestamp": 1234567890,
  "total_agents": 5,
  "active_agents": 4,
  "total_tasks": 10,
  "completed_tasks": 8,
  "failed_tasks": 1,
  "pending_tasks": 1,
  "network_latency_ms": 15.5,
  "message_rate": 120.5,
  "consensus_rounds": 45,
  "avg_consensus_time_ms": 18.2
}
```

### Agents

#### List all agents
```
GET /api/agents
```

#### Get agent details
```
GET /api/agents/:id
```

#### Update agent
```
PUT /api/agents/:id
```
**Body:**
```json
{
  "id": "agent-1",
  "status": "active",
  "capabilities": ["navigation", "lidar"],
  "resources": {
    "cpu_percent": 45.5,
    "memory_percent": 60.2,
    "battery_level": 85.0
  }
}
```

### Tasks

#### List all tasks
```
GET /api/tasks
```

#### Create task
```
POST /api/tasks
```
**Body:**
```json
{
  "description": "Explore warehouse zone A",
  "priority": 150,
  "required_capabilities": ["navigation", "lidar"],
  "deadline": 3600,
  "dependencies": ["task-1", "task-2"]
}
```

**Response:**
```json
{
  "id": "uuid",
  "description": "Explore warehouse zone A",
  "status": "pending",
  "priority": 150,
  "assigned_agent": null,
  "progress": 0.0,
  "created_at": 1234567890
}
```

#### Get task details
```
GET /api/tasks/:id
```

#### Assign task to agent
```
POST /api/tasks/:id/assign
```
**Body:**
```json
{
  "task_id": "uuid",
  "agent_id": "agent-1"
}
```

#### Cancel task
```
POST /api/tasks/:id/cancel
```

### Workflows

#### List all workflows
```
GET /api/workflows
```

#### Start workflow
```
POST /api/workflows/start
```
**Body:**
```json
{
  "workflow_id": "exploration_workflow",
  "parameters": {
    "warehouse_id": "warehouse_001",
    "robot_count": "4"
  }
}
```

#### Get workflow details
```
GET /api/workflows/:id
```

#### Pause workflow
```
POST /api/workflows/:id/pause
```

#### Resume workflow
```
POST /api/workflows/:id/resume
```

#### Cancel workflow
```
POST /api/workflows/:id/cancel
```

### WebSocket

Connect to WebSocket for real-time updates:
```
WS /ws
```

**Message Types:**

#### Agent Status Update
```json
{
  "type": "agent_status",
  "agent_id": "agent-1",
  "status": {
    "agent_id": "agent-1",
    "state": "active",
    "battery_level": 85.0,
    "connected_peers": 3
  }
}
```

#### Task Created
```json
{
  "type": "task_created",
  "id": "uuid",
  "description": "New task",
  "status": "pending",
  "priority": 100
}
```

#### Task Updated
```json
{
  "type": "task_updated",
  "id": "uuid",
  "description": "Updated task",
  "status": "assigned",
  "assigned_agent": "agent-1",
  "progress": 25.0
}
```

#### Workflow Started
```json
{
  "type": "workflow_started",
  "id": "uuid",
  "name": "Exploration Workflow",
  "status": "running",
  "progress": 0.0
}
```

#### Metrics Update
```json
{
  "type": "metrics_update",
  "total_agents": 5,
  "active_agents": 4,
  "total_tasks": 10,
  "completed_tasks": 8,
  "network_latency_ms": 15.5
}
```

## Prometheus Metrics

### Counters
- `sdk_tasks_completed_total` - Total completed tasks
- `sdk_tasks_failed_total` - Total failed tasks
- `sdk_tasks_pending_total` - Total pending tasks
- `sdk_messages_sent_total` - Total messages sent
- `sdk_messages_received_total` - Total messages received
- `sdk_consensus_rounds_total` - Total consensus rounds
- `sdk_consensus_success_total` - Successful consensus rounds
- `sdk_consensus_timeout_total` - Consensus timeouts

### Gauges
- `sdk_active_agents` - Number of active agents
- `sdk_connected_peers` - Number of connected peers
- `sdk_crdt_keys_count` - Number of keys in CRDT map
- `sdk_workflow_instances` - Number of active workflows
- `sdk_agent_battery_level` - Agent battery level
- `sdk_cpu_usage_percent` - CPU usage
- `sdk_memory_usage_percent` - Memory usage

### Histograms
- `sdk_message_latency_ms` - Message latency distribution
- `sdk_consensus_time_ms` - Consensus round time distribution
- `sdk_task_duration_secs` - Task duration distribution
- `sdk_sync_duration_ms` - State sync duration distribution

## Usage Example

### Rust Backend

```rust
use dashboard::{start_dashboard, ApiState, routes};

#[tokio::main]
async fn main() {
    let state = ApiState::new();
    let dashboard = routes(state);
    
    warp::serve(dashboard)
        .run("0.0.0.0:3000".parse().unwrap())
        .await;
}
```

### Python Client

```python
import requests
import websocket
import json

# REST API
response = requests.get('http://localhost:3000/api/health')
print(response.json())

# Create task
task = {
    'description': 'Explore zone A',
    'priority': 150
}
response = requests.post(
    'http://localhost:3000/api/tasks',
    json=task
)
task_id = response.json()['id']

# WebSocket for real-time updates
def on_message(ws, message):
    data = json.loads(message)
    print(f"Received: {data}")

ws = websocket.WebSocketApp("ws://localhost:3000/ws",
                            on_message=on_message)
ws.run_forever()
```

### cURL Examples

```bash
# Health check
curl http://localhost:3000/api/health

# List agents
curl http://localhost:3000/api/agents

# Create task
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{"description": "Test task", "priority": 150}'

# Assign task
curl -X POST http://localhost:3000/api/tasks/<task_id>/assign \
  -H "Content-Type: application/json" \
  -d '{"agent_id": "agent-1"}'

# Get metrics
curl http://localhost:3000/api/metrics

# Prometheus metrics
curl http://localhost:3000/metrics
```

## Frontend Development

### Prerequisites
- Rust toolchain
- [trunk](https://trunkrs.dev/) - WASM build tool
- Node.js (for dev server)

### Build and Run

```bash
# Install dependencies
trunk serve

# Production build
trunk build --release
```

### Components

- `AgentList` - Display all agents and their status
- `TaskList` - Show tasks with filtering and sorting
- `NetworkGraph` - Visualize mesh network topology
- `MetricsPanel` - Show system metrics and health
- `TaskDetails` - Detailed task view and actions
- `WorkflowManager` - Workflow control interface

## Configuration

Environment variables:
```bash
DASHBOARD_BIND_ADDRESS=0.0.0.0:3000
DASHBOARD_METRICS_ENABLED=true
DASHBOARD_WEBSOCKET_ENABLED=true
DASHBOARD_LOG_LEVEL=info
```

## Security

- HTTPS support (via TLS certificates)
- Authentication via JWT tokens
- Rate limiting
- CORS configuration
- Input validation

## Performance

- **Latency**: <10ms for REST API
- **Throughput**: 10,000+ req/s
- **WebSocket**: 1000+ concurrent connections
- **Memory**: <100MB for typical deployment

## Troubleshooting

### Port already in use
```bash
# Find process using port 3000
lsof -i :3000

# Kill process
kill -9 <PID>
```

### WebSocket connection fails
- Check firewall settings
- Verify CORS configuration
- Check browser console for errors

### Metrics not appearing
- Ensure Prometheus is configured correctly
- Check `/metrics` endpoint directly
- Verify metric registration in code

## Contributing

See main repository CONTRIBUTING.md for guidelines.

## License

MIT OR Apache-2.0
