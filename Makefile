# Makefile for Offline-First Multi-Agent Autonomy SDK

.PHONY: help build test clean docs python-bindings docker ros2

help: ## Show this help message
	@echo "Usage: make [target]"
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

## Build Targets
## ============

build: ## Build all Rust crates in release mode
	cargo build --release --workspace

build-debug: ## Build all crates in debug mode
	cargo build --workspace

build-python: python-bindings ## Build Python bindings
	@echo "Python bindings built successfully"

python-bindings:
	cd crates/python-bindings && maturin develop --release

python-bindings-debug:
	cd crates/python-bindings && maturin develop

## Testing
## =======

test: ## Run all tests
	cargo test --workspace

test-coverage: ## Run tests with coverage report
	cargo tarpaulin --workspace --out Html

test-python: ## Run Python tests
	pytest crates/python-bindings/tests -v

test-fuzz: ## Run fuzz tests (requires cargo-fuzz)
	cd crates/state-sync && cargo fuzz run crdt_merge

## Clean
## =====

clean: ## Clean build artifacts
	cargo clean
	rm -rf target/
	rm -rf __pycache__/
	rm -rf *.egg-info/

clean-python: ## Clean Python build artifacts
	cd crates/python-bindings && maturin clean
	find . -type d -name "__pycache__" -exec rm -rf {} +
	find . -type f -name "*.pyc" -delete

## Documentation
## =============

docs: ## Generate documentation
	cargo doc --workspace --no-deps --open

docs-python: ## Build Python documentation
	mkdocs build

docs-serve: ## Serve documentation locally
	mkdocs serve

## Python
## ======

python-install: ## Install Python dependencies
	pip install -r python-requirements.txt

python-lint: ## Run Python linters
	flake8 examples/
	mypy examples/

python-format: ## Format Python code
	black examples/
	isort examples/

python-demo: ## Run Python demo
	python3 examples/python_demo.py

## Docker
## ======

docker-build: ## Build Docker image
	docker build -t multi-agent-sdk .

docker-run: ## Run Docker container
	docker run -p 3000:3000 multi-agent-sdk

docker-dev: ## Build and run in development mode
	docker-compose -f docker-compose.dev.yml up

## ROS2
## ====

ros2-setup: ## Setup ROS2 environment
	source /opt/ros/humble/setup.bash
	catkin_make -DCMAKE_BUILD_TYPE=Release

ros2-sim: ## Run ROS2 Gazebo simulation
	source /opt/ros/humble/setup.bash
	roslaunch examples ros2_gazebo_simulation.launch

ros2-demo: ## Run ROS2 demo scenarios
	source /opt/ros/humble/setup.bash
	roslaunch examples search_and_rescue.launch

## CI/CD
## =====

ci: build test docs ## Run all CI checks

ci-python: python-bindings test-python python-lint ## Run Python CI

ci-all: ci ci-python ## Run complete CI pipeline

## Development
## ===========

dev-dashboard: ## Start dashboard server
	cargo run --example dashboard_server --release

dev-monitor: ## Start monitoring with Prometheus
	# Requires Prometheus installed
	prometheus --config.prometheus.yml

dev-grafana: ## Start Grafana for visualization
	grafana-server --config grafana.ini

## Release
## =======

release: ## Prepare for release
	cargo release --workspace

release-python: ## Build Python wheel for release
	cd crates/python-bindings && maturin build --release

## Utilities
## =========

fmt: ## Format Rust code
	cargo fmt --all

clippy: ## Run Clippy linter
	cargo clippy --workspace -- -D warnings

audit: ## Run security audit
	cargo audit

bench: ## Run benchmarks
	cargo bench
