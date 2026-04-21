#!/bin/bash
# Local testing and simulation script
#
# Usage: ./scripts/local_test.sh [OPTIONS]
#   --full           Run all tests (default)
#   --quick          Run only unit tests
#   --integration    Run only integration tests
#   --bench          Run benchmarks
#   --fuzz           Run fuzz tests (requires cargo-fuzz)
#   --ros2           Run ROS2 integration tests
#   --help           Show this help

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Offline-First Multi-Agent Autonomy SDK - Local Test Suite ===${NC}"
echo ""

# Default mode
MODE="full"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --full)
            MODE="full"
            shift
            ;;
        --quick)
            MODE="quick"
            shift
            ;;
        --integration)
            MODE="integration"
            shift
            ;;
        --bench)
            MODE="bench"
            shift
            ;;
        --fuzz)
            MODE="fuzz"
            shift
            ;;
        --ros2)
            MODE="ros2"
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --full        Run all tests (default)"
            echo "  --quick       Run only unit tests"
            echo "  --integration Run only integration tests"
            echo "  --bench       Run benchmarks"
            echo "  --fuzz        Run fuzz tests (requires cargo-fuzz)"
            echo "  --ros2        Run ROS2 integration tests"
            echo "  --help        Show this help"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Function to run unit tests
run_unit_tests() {
    echo -e "${YELLOW}[1/4] Running unit tests...${NC}"
    cargo test --workspace --lib --verbose
    echo -e "${GREEN}✓ Unit tests passed${NC}"
}

# Function to run integration tests
run_integration_tests() {
    echo -e "${YELLOW}[2/4] Running integration tests...${NC}"
    cargo test --workspace --test '*' --features integration-tests --verbose
    echo -e "${GREEN}✓ Integration tests passed${NC}"
}

# Function to run code quality checks
run_quality_checks() {
    echo -e "${YELLOW}[3/4] Running code quality checks...${NC}"
    
    # Format check
    cargo fmt --all -- --check
    
    # Clippy
    cargo clippy --workspace --all-targets -- -D warnings
    
    echo -e "${GREEN}✓ Code quality checks passed${NC}"
}

# Function to run benchmarks
run_benchmarks() {
    echo -e "${YELLOW}[4/4] Running benchmarks...${NC}"
    cargo bench --workspace --no-run
    cargo bench --workspace -- --quick
    
    echo -e "${GREEN}✓ Benchmarks completed${NC}"
}

# Function to run fuzz tests
run_fuzz_tests() {
    echo -e "${YELLOW}Running fuzz tests (this may take a while)...${NC}"
    
    if ! command -v cargo-fuzz &> /dev/null; then
        echo -e "${RED}cargo-fuzz not found. Install with: cargo install cargo-fuzz${NC}"
        exit 1
    fi
    
    # Run fuzz tests for limited time
    cd crates/state-sync
    cargo fuzz run crdt_merge_fuzzer -- -max_total_time=60
    cargo fuzz run delta_serialization_fuzzer -- -max_total_time=60
    cd ../..
    
    cd crates/mesh-transport
    cargo fuzz run message_serialization_fuzzer -- -max_total_time=60
    cd ../..
    
    echo -e "${GREEN}✓ Fuzz tests completed${NC}"
}

# Function to run ROS2 tests
run_ros2_tests() {
    echo -e "${YELLOW}Running ROS2 integration tests...${NC}"
    
    # Check if ROS2 is installed
    if [ -f "/opt/ros/humble/setup.bash" ]; then
        source /opt/ros/humble/setup.bash
        
        # Build ROS2 workspace
        cd examples/ros2_gazebo
        colcon build --packages-select ros2_gazebo
        
        # Run tests
        colcon test --packages-select ros2_gazebo
        colcon test-result --verbose
        
        cd ../..
        echo -e "${GREEN}✓ ROS2 tests completed${NC}"
    else
        echo -e "${YELLOW}ROS2 not found. Skipping ROS2 tests.${NC}"
        echo "Install ROS2 Humble to run these tests:"
        echo "  sudo apt install ros-humble-desktop"
    fi
}

# Function to run security audit
run_security_audit() {
    echo -e "${YELLOW}Running security audit...${NC}"
    
    if ! command -v cargo-audit &> /dev/null; then
        echo -e "${YELLOW}cargo-audit not found. Skipping security audit.${NC}"
        echo "Install with: cargo install cargo-audit"
        return
    fi
    
    cargo audit
    
    echo -e "${GREEN}✓ Security audit passed${NC}"
}

# Main execution
case $MODE in
    quick)
        run_unit_tests
        ;;
    
    integration)
        run_unit_tests
        run_integration_tests
        ;;
    
    bench)
        run_benchmarks
        ;;
    
    fuzz)
        run_fuzz_tests
        ;;
    
    ros2)
        run_ros2_tests
        ;;
    
    full)
        run_unit_tests
        run_integration_tests
        run_quality_checks
        run_security_audit
        echo ""
        echo -e "${GREEN}=== All tests passed! ===${NC}"
        ;;
esac

echo ""
echo -e "${GREEN}Test execution completed!${NC}"
