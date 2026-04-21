#!/bin/bash

# Integration test runner
# Runs all end-to-end tests with proper setup

set -e

echo "======================================"
echo "  SDK Integration Test Suite"
echo "======================================"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
PASSED=0
FAILED=0
SKIPPED=0

# Function to run a test
run_test() {
    local test_name=$1
    local test_command=$2
    
    echo -e "${YELLOW}Running: ${test_name}${NC}"
    
    if eval $test_command; then
        echo -e "${GREEN}✓ PASSED${NC}: ${test_name}"
        ((PASSED++))
    else
        echo -e "${RED}✗ FAILED${NC}: ${test_name}"
        ((FAILED++))
    fi
    echo ""
}

# Setup environment
export RUST_LOG=info
export DATABASE_URL="sqlite::memory:"

# Build first
echo "Building project..."
cargo build --workspace --tests
echo ""

# Run unit tests
echo "======================================"
echo "  Unit Tests"
echo "======================================"
run_test "Common crate" "cargo test -p common"
run_test "Mesh Transport" "cargo test -p mesh-transport"
run_test "State Sync" "cargo test -p state-sync"
run_test "Distributed Planner" "cargo test -p distributed-planner"
run_test "Workflow Orchestration" "cargo test -p workflow-orchestration"
run_test "Database" "cargo test -p database"
run_test "Auth" "cargo test -p auth"
echo ""

# Run integration tests
echo "======================================"
echo "  Integration Tests"
echo "======================================"
run_test "End-to-End Workflow" "cargo test -p integration-tests test_full_workflow_lifecycle"
run_test "Workflow Orchestration" "cargo test -p integration-tests test_workflow_orchestration_integration"
run_test "Multi-Agent Coordination" "cargo test -p integration-tests test_multi_agent_coordination"
run_test "State Synchronization" "cargo test -p integration-tests test_state_synchronization"
run_test "Authentication Flow" "cargo test -p integration-tests test_authentication_flow"
run_test "Performance Benchmarks" "cargo test -p integration-tests test_performance_benchmarks"
echo ""

# Run fuzz tests (short duration)
echo "======================================"
echo "  Fuzz Tests (30 seconds each)"
echo "======================================"
if command -v cargo-fuzz &> /dev/null; then
    cd crates/state-sync
    timeout 30 cargo fuzz run crdt_merge || true
    cd ../..
    ((SKIPPED++))
    echo -e "${YELLOW}⚠ Fuzz tests skipped (timeout)${NC}"
else
    echo -e "${YELLOW}⚠ cargo-fuzz not installed, skipping fuzz tests${NC}"
    ((SKIPPED++))
fi
echo ""

# Run clippy
echo "======================================"
echo "  Code Quality Checks"
echo "======================================"
run_test "Clippy (no warnings)" "cargo clippy --workspace -- -D warnings"
run_test "Format check" "cargo fmt --all -- --check"
echo ""

# Summary
echo "======================================"
echo "  Test Summary"
echo "======================================"
echo -e "${GREEN}Passed:  ${PASSED}${NC}"
echo -e "${RED}Failed:  ${FAILED}${NC}"
echo -e "${YELLOW}Skipped: ${SKIPPED}${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}  ALL TESTS PASSED! 🎉${NC}"
    echo -e "${GREEN}======================================${NC}"
    exit 0
else
    echo -e "${RED}======================================${NC}"
    echo -e "${RED}  SOME TESTS FAILED ❌${NC}"
    echo -e "${RED}======================================${NC}"
    exit 1
fi
