# CLI Reference Guide

## Overview

The SDK CLI (`sdk`) provides a powerful command-line interface for managing the Multi-Agent Autonomy SDK from the terminal.

## Installation

### From Source

```bash
cd crates/cli
cargo build --release
```

The binary will be available at `target/release/sdk`.

### From Pre-built Binaries

```bash
# Download for your platform
curl -LO https://github.com/your-org/sdk/releases/latest/download/sdk-linux-x86_64

# Make executable
chmod +x sdk-linux-x86_64

# Move to PATH
sudo mv sdk-linux-x86_64 /usr/local/bin/sdk
```

## Usage

```bash
sdk [OPTIONS] <COMMAND>

Global Options:
  -a, --api <URL>       API endpoint (default: http://localhost:3000)
  -f, --format <FORMAT> Output format: table, json, yaml (default: table)
  -v, --verbose         Verbose output
  -h, --help            Print help
  -V, --version         Print version
```

## Commands

### Task Management

#### List Tasks

```bash
# List all tasks
sdk task list

# Filter by status
sdk task list --status pending

# Limit results
sdk task list --limit 20
```

#### Create Task

```bash
# Basic task
sdk task create --description "Explore zone A"

# With priority
sdk task create --description "Scan area" --priority 150

# With required capabilities
sdk task create --description "Image analysis" \
  --capabilities vision processing
```

#### Get Task Details

```bash
sdk task get --id task-123
```

#### Update Task

```bash
# Update status
sdk task update --id task-123 --status running

# Assign to agent
sdk task update --id task-123 --agent agent-1

# Update both
sdk task update --id task-123 --status completed --agent agent-1
```

#### Delete Task

```bash
sdk task delete --id task-123
```

### Agent Management

#### List Agents

```bash
# List all agents
sdk agent list

# Filter by status
sdk agent list --status online
```

#### Get Agent Details

```bash
sdk agent get --id agent-1
```

#### Register Agent

```bash
sdk agent register --name "exploration-bot" \
  --capabilities navigation lidar mapping
```

#### Unregister Agent

```bash
sdk agent unregister --id agent-1
```

### Workflow Management

#### List Workflows

```bash
sdk workflow list
```

#### Create Workflow

```bash
# From YAML file
sdk workflow create --name "warehouse-workflow" \
  --file workflow.yaml

# Without definition
sdk workflow create --name "simple-workflow"
```

#### Start Workflow

```bash
sdk workflow start --id workflow-123
```

#### Get Workflow Status

```bash
sdk workflow status --id instance-456
```

### System Operations

#### Health Check

```bash
sdk system health
```

Example output:
```
✅ Health Check:

  Status:   ok
  Version:  1.0.0
  Timestamp: 2026-03-27T10:30:00Z
```

#### Get Metrics

```bash
sdk system metrics
```

Example output:
```
📊 System Metrics:

  Total Agents:     5
  Active Agents:    4
  Total Tasks:      150
  Completed Tasks:  120
  Failed Tasks:     3
  Pending Tasks:    27
  Network Latency:  45.23 ms
  Message Rate:     1250.50 msg/s
```

#### Get Statistics

```bash
sdk system stats
```

Example output:
```
📈 System Statistics:

  Uptime:           7d 14h 32m
  Total Requests:   125,430
  Avg Response Time: 8.45 ms
  Active Connections: 23
  Memory Usage:     256 MB
  CPU Usage:        45.2%
```

#### Interactive Mode

```bash
sdk system interactive
```

Available commands in interactive mode:
- `health` - Health check
- `metrics` - System metrics
- `stats` - System statistics
- `tasks` - List tasks
- `agents` - List agents
- `quit` - Exit

Example:
```
🎮 Interactive Mode - Type 'quit' to exit

sdk> health
✅ Health Check:
  Status: ok
  Version: 1.0.0

sdk> metrics
📊 System Metrics:
  Total Agents: 5
  ...

sdk> quit
Goodbye!
```

### Configuration

#### Show Configuration

```bash
sdk config show
```

#### Set Configuration

```bash
# Set API endpoint
sdk config set api.url http://api.example.com

# Set timeout
sdk config set network.timeout 30

# Nested configuration
sdk config set logging.level debug
```

#### Reset Configuration

```bash
sdk config reset
```

## Examples

### Quick Start

```bash
# Check system health
sdk system health

# List agents
sdk agent list

# Create a task
sdk task create --description "Explore warehouse" --priority 150

# Monitor metrics
sdk system metrics
```

### Workflow Automation

```bash
#!/bin/bash

# Register new agent
sdk agent register --name "worker-1" \
  --capabilities navigation manipulation

# Create workflow
sdk workflow create --name "pick-and-place" \
  --file pick_place_workflow.yaml

# Start workflow
sdk workflow start --id $(sdk workflow list | grep pick-and-place | awk '{print $1}')

# Monitor progress
watch -n 5 'sdk workflow status --id INSTANCE_ID'
```

### Task Batch Operations

```bash
# Create multiple tasks
for i in {1..10}; do
  sdk task create --description "Task $i" --priority $((100 + i * 10))
done

# List high-priority tasks
sdk task list --limit 50 | grep -E "(1[1-9][0-9]|2[0-9][0-9]|300)"
```

### JSON Output

```bash
# Get task as JSON
sdk task get --id task-123 --format json

# Parse with jq
sdk task list --format json | jq '.[] | select(.status == "pending")'
```

## Output Formats

### Table (Default)

Human-readable tabular format, ideal for terminal viewing.

### JSON

Machine-readable JSON format for scripting and automation.

```bash
sdk task list --format json
```

### YAML

Human-readable structured format.

```bash
sdk task list --format yaml
```

## Error Handling

### Common Errors

**Connection Failed:**
```
Error: Failed to connect to API at http://localhost:3000
Hint: Make sure the SDK dashboard is running
```

**Task Not Found:**
```
Error: Task with ID 'task-123' not found
```

**Invalid Priority:**
```
Error: Priority must be between 1 and 1000
```

### Debug Mode

Enable debug output:

```bash
RUST_LOG=debug sdk task list
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SDK_API_URL` | API endpoint | `http://localhost:3000` |
| `SDK_OUTPUT_FORMAT` | Output format | `table` |
| `RUST_LOG` | Log level | `info` |

Example:
```bash
export SDK_API_URL=http://api.example.com
sdk task list
```

## Keyboard Shortcuts

In interactive mode:
- `Ctrl+C` - Cancel current command
- `Ctrl+D` - Exit
- `Tab` - Autocomplete

## Configuration File

The CLI stores configuration at:
- Linux: `~/.config/sdk/config.json`
- macOS: `~/Library/Application Support/sdk/config.json`
- Windows: `%APPDATA%/sdk/config.json`

Example config:
```json
{
  "api": {
    "url": "http://localhost:3000",
    "timeout": 30
  },
  "output": {
    "format": "table",
    "color": true
  },
  "logging": {
    "level": "info"
  }
}
```

## Completion Scripts

Generate shell completion:

```bash
# Bash
sdk completion bash > /etc/bash_completion.d/sdk

# Zsh
sdk completion zsh > /usr/local/share/zsh/site-functions/_sdk

# Fish
sdk completion fish > ~/.config/fish/completions/sdk.fish
```

## Troubleshooting

### CLI Won't Start

```bash
# Check installation
which sdk

# Check version
sdk --version

# Reinstall if needed
cargo install --path crates/cli
```

### API Connection Issues

```bash
# Verify API is running
curl http://localhost:3000/api/health

# Check firewall
sudo ufw status

# Test with different endpoint
sdk --api http://192.168.1.100:3000 task list
```

### Permission Errors

```bash
# Make executable
chmod +x $(which sdk)

# Check ownership
ls -la $(which sdk)
```

## Best Practices

1. **Use Environment Variables** - For API URL and configuration
2. **Enable Autocomplete** - Improves productivity
3. **Use JSON for Scripts** - Easier to parse
4. **Batch Operations** - Use loops for multiple tasks
5. **Error Handling** - Check exit codes in scripts
6. **Logging** - Use `--verbose` for debugging

## Next Steps

- [CLI API Reference](./CLI_API_REFERENCE.md)
- [Configuration Guide](./CLI_CONFIG.md)
- [Scripting Examples](./CLI_SCRIPTING.md)

---

*Last Updated: 2026-03-27*
