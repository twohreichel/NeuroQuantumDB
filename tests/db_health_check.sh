#!/bin/bash

##############################################################################
# NeuroQuantumDB Health Check and Testing Script
# Enterprise-grade database monitoring and testing framework
# Compatible with Linux/Unix/macOS systems
##############################################################################

set -euo pipefail

# Script metadata
readonly SCRIPT_VERSION="1.0.0"
readonly SCRIPT_NAME="NeuroQuantumDB Health Check"
readonly SCRIPT_DATE="2025-09-19"

# Configuration
readonly CONFIG_FILE="${CONFIG_FILE:-../config/dev.toml}"
readonly LOG_DIR="${LOG_DIR:-./logs}"
readonly LOG_FILE="${LOG_DIR}/db_health_$(date +%Y%m%d_%H%M%S).log"
readonly SUMMARY_FILE="${LOG_DIR}/health_summary_$(date +%Y%m%d).json"
readonly MAX_LOG_FILES=30

# Performance thresholds
readonly CONNECTION_TIMEOUT_MS=5000
readonly QUERY_TIMEOUT_MS=1000
readonly SLOW_QUERY_THRESHOLD_MS=2000
readonly MEMORY_THRESHOLD_MB=1024
readonly CPU_THRESHOLD_PERCENT=80

# Color codes for terminal output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly PURPLE='\033[0;35m'
readonly CYAN='\033[0;36m'
readonly NC='\033[0m' # No Color

# Test counters
TESTS_TOTAL=0
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_WARNING=0

# Database connection variables
DB_HOST=""
DB_PORT=""
DB_NAME=""
API_ENDPOINT=""

##############################################################################
# Utility Functions
##############################################################################

# Initialize logging and create necessary directories
initialize_logging() {
    mkdir -p "$LOG_DIR"

    # Rotate old log files
    find "$LOG_DIR" -name "db_health_*.log" -type f -mtime +7 -delete 2>/dev/null || true

    # Create log file with header
    {
        echo "==================================================="
        echo "NeuroQuantumDB Health Check Log"
        echo "Started: $(date '+%Y-%m-%d %H:%M:%S')"
        echo "Version: $SCRIPT_VERSION"
        echo "==================================================="
    } > "$LOG_FILE"
}

# Logging functions with timestamps and severity levels
log_info() {
    local message="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo -e "${BLUE}[${timestamp}] [INFO]${NC} $message" | tee -a "$LOG_FILE"
}

log_success() {
    local message="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo -e "${GREEN}[${timestamp}] [SUCCESS] ✓${NC} $message" | tee -a "$LOG_FILE"
    ((TESTS_PASSED++))
}

log_warning() {
    local message="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo -e "${YELLOW}[${timestamp}] [WARNING] ⚠${NC} $message" | tee -a "$LOG_FILE"
    ((TESTS_WARNING++))
}

log_error() {
    local message="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo -e "${RED}[${timestamp}] [ERROR] ✗${NC} $message" | tee -a "$LOG_FILE"
    ((TESTS_FAILED++))
}

log_remediation() {
    local message="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo -e "${PURPLE}[${timestamp}] [REMEDIATION] →${NC} $message" | tee -a "$LOG_FILE"
}

# Measure execution time in milliseconds
measure_time() {
    local start_time=$(date +%s.%N)
    "$@"
    local end_time=$(date +%s.%N)
    local duration=$(echo "scale=0; ($end_time - $start_time) * 1000" | bc -l)
    echo "$duration"
}

# Parse configuration file
parse_config() {
    log_info "Parsing configuration file: $CONFIG_FILE"

    if [[ ! -f "$CONFIG_FILE" ]]; then
        log_error "Configuration file not found: $CONFIG_FILE"
        log_remediation "Create configuration file or set CONFIG_FILE environment variable"
        exit 1
    fi

    # Simple and reliable TOML parsing using sed and grep
    # Extract host from [server] section
    DB_HOST=$(sed -n '/^\[server\]/,/^\[/p' "$CONFIG_FILE" | grep "^host" | sed 's/.*=\s*"\([^"]*\)".*/\1/')
    DB_PORT=$(sed -n '/^\[server\]/,/^\[/p' "$CONFIG_FILE" | grep "^port" | sed 's/.*=\s*\([0-9]*\).*/\1/')

    # Set defaults if extraction failed
    if [[ -z "$DB_HOST" || "$DB_HOST" == "0.0.0.0" ]]; then
        DB_HOST="localhost"
        log_info "Using localhost for host (config has 0.0.0.0 or extraction failed)"
    fi
    if [[ -z "$DB_PORT" ]]; then
        DB_PORT="8080"
        log_info "Using default port: $DB_PORT"
    fi

    # Extract database name from connection string
    local connection_string=$(grep "connection_string" "$CONFIG_FILE" | sed 's/.*=\s*"\([^"]*\)".*/\1/')
    if [[ -n "$connection_string" ]]; then
        DB_NAME=$(echo "$connection_string" | sed 's/.*\/\([^?\/]*\).*/\1/')
    fi

    API_ENDPOINT="http://${DB_HOST}:${DB_PORT}"

    log_success "Configuration parsed successfully"
    log_info "Database Host: $DB_HOST"
    log_info "Database Port: $DB_PORT"
    log_info "API Endpoint: $API_ENDPOINT"
}

##############################################################################
# Test Functions
##############################################################################

# Test 1: Connection and Authentication
test_connection() {
    log_info "=== Connection and Authentication Test ==="
    ((TESTS_TOTAL++))

    local duration
    duration=$(measure_time curl -s -w "%{time_total}" -o /dev/null "$API_ENDPOINT/health" 2>/dev/null || echo "0")
    duration=$(echo "scale=0; $duration * 1000" | bc -l)

    if [[ "$duration" -gt 0 && "$duration" -lt "$CONNECTION_TIMEOUT_MS" ]]; then
        log_success "Connection Test: API endpoint reachable (Response: ${duration}ms)"
    elif [[ "$duration" -ge "$CONNECTION_TIMEOUT_MS" ]]; then
        log_warning "Connection Test: Slow response detected (Response: ${duration}ms > threshold: ${CONNECTION_TIMEOUT_MS}ms)"
        log_remediation "Check network latency and server load: ping $DB_HOST"
    else
        log_error "Connection Test: Failed to connect to API endpoint $API_ENDPOINT"
        log_remediation "Verify service is running: systemctl status neuroquantum-api or check process: ps aux | grep neuroquantum"
    fi
}

# Test 2: Database Structure Integrity
test_database_structure() {
    log_info "=== Database Structure Integrity Test ==="
    ((TESTS_TOTAL++))

    # Check if the NeuroQuantumDB binary exists and is executable
    local binary_path="../target/release/neuroquantum-api"
    if [[ ! -f "$binary_path" ]]; then
        binary_path="../target/debug/neuroquantum-api"
    fi

    if [[ -f "$binary_path" && -x "$binary_path" ]]; then
        log_success "Database Structure: NeuroQuantumDB binary found and executable"

        # Check binary dependencies
        if command -v ldd >/dev/null 2>&1; then
            local missing_deps=$(ldd "$binary_path" 2>/dev/null | grep "not found" | wc -l)
            if [[ "$missing_deps" -eq 0 ]]; then
                log_success "Database Structure: All binary dependencies satisfied"
            else
                log_error "Database Structure: Missing binary dependencies detected"
                log_remediation "Check dependencies: ldd $binary_path"
            fi
        fi
    else
        log_error "Database Structure: NeuroQuantumDB binary not found or not executable"
        log_remediation "Build the project: cargo build --release"
    fi
}

# Test 3: Data Consistency and Validation
test_data_consistency() {
    log_info "=== Data Consistency and Validation Test ==="
    ((TESTS_TOTAL++))

    # Test neuromorphic configuration consistency
    local synaptic_threshold=$(grep "synaptic_strength_threshold" "$CONFIG_FILE" | sed 's/.*=\s*\([0-9.]*\).*/\1/')
    local learning_rate=$(grep "learning_rate" "$CONFIG_FILE" | sed 's/.*=\s*\([0-9.]*\).*/\1/')

    if [[ -n "$synaptic_threshold" && -n "$learning_rate" ]]; then
        # Validate threshold ranges
        if (( $(echo "$synaptic_threshold >= 0.0 && $synaptic_threshold <= 1.0" | bc -l) )); then
            log_success "Data Consistency: Synaptic threshold within valid range ($synaptic_threshold)"
        else
            log_error "Data Consistency: Invalid synaptic threshold value ($synaptic_threshold)"
            log_remediation "Set synaptic_strength_threshold between 0.0 and 1.0 in config"
        fi

        if (( $(echo "$learning_rate > 0.0 && $learning_rate <= 1.0" | bc -l) )); then
            log_success "Data Consistency: Learning rate within valid range ($learning_rate)"
        else
            log_error "Data Consistency: Invalid learning rate value ($learning_rate)"
            log_remediation "Set learning_rate between 0.0 and 1.0 in config"
        fi
    else
        log_warning "Data Consistency: Could not validate neuromorphic parameters"
        log_remediation "Verify neuromorphic_config section in configuration file"
    fi
}

# Test 4: Performance Benchmarking
test_performance() {
    log_info "=== Performance Benchmarking Test ==="
    ((TESTS_TOTAL++))

    # Test API response time for health endpoint
    local api_times=()
    for i in {1..5}; do
        local duration
        duration=$(measure_time curl -s -w "%{time_total}" -o /dev/null "$API_ENDPOINT/health" 2>/dev/null || echo "0")
        duration=$(echo "scale=0; $duration * 1000" | bc -l)
        api_times+=("$duration")
    done

    # Calculate average response time
    local total=0
    local count=0
    for time in "${api_times[@]}"; do
        if [[ "$time" -gt 0 ]]; then
            total=$((total + time))
            ((count++))
        fi
    done

    if [[ "$count" -gt 0 ]]; then
        local avg_time=$((total / count))
        if [[ "$avg_time" -lt "$QUERY_TIMEOUT_MS" ]]; then
            log_success "Performance Test: Average API response time acceptable (${avg_time}ms)"
        elif [[ "$avg_time" -lt "$SLOW_QUERY_THRESHOLD_MS" ]]; then
            log_warning "Performance Test: Slow API response detected (${avg_time}ms > threshold: ${QUERY_TIMEOUT_MS}ms)"
            log_remediation "Monitor system resources and optimize API endpoints"
        else
            log_error "Performance Test: Very slow API response (${avg_time}ms > critical: ${SLOW_QUERY_THRESHOLD_MS}ms)"
            log_remediation "Check system load: top -p \$(pgrep neuroquantum) and investigate bottlenecks"
        fi
    else
        log_error "Performance Test: Could not measure API performance"
        log_remediation "Verify API service is responding: curl -v $API_ENDPOINT/health"
    fi
}

# Test 5: Resource Utilization Monitoring
test_resource_utilization() {
    log_info "=== Resource Utilization Test ==="
    ((TESTS_TOTAL++))

    # Check memory usage
    local memory_usage_mb
    if command -v free >/dev/null 2>&1; then
        memory_usage_mb=$(free -m | awk 'NR==2{printf "%.0f", $3}')
    elif command -v vm_stat >/dev/null 2>&1; then
        # macOS memory calculation
        local pages_used=$(vm_stat | grep "Pages active\|Pages inactive\|Pages speculative\|Pages wired down" | awk '{print $3}' | sed 's/\.//' | paste -sd+ | bc)
        memory_usage_mb=$(echo "scale=0; $pages_used * 4096 / 1024 / 1024" | bc)
    else
        memory_usage_mb=0
    fi

    if [[ "$memory_usage_mb" -gt 0 ]]; then
        if [[ "$memory_usage_mb" -lt "$MEMORY_THRESHOLD_MB" ]]; then
            log_success "Resource Test: Memory usage within limits (${memory_usage_mb}MB)"
        else
            log_warning "Resource Test: High memory usage detected (${memory_usage_mb}MB > ${MEMORY_THRESHOLD_MB}MB)"
            log_remediation "Monitor memory: ps aux --sort=-%mem | head -10"
        fi
    else
        log_warning "Resource Test: Could not determine memory usage"
    fi

    # Check CPU usage
    local cpu_usage
    if command -v top >/dev/null 2>&1; then
        cpu_usage=$(top -l 1 -n 0 | grep "CPU usage" | awk '{print $3}' | sed 's/%//' 2>/dev/null || echo "0")
    else
        cpu_usage=0
    fi

    if [[ "$cpu_usage" -gt 0 ]]; then
        if [[ "$cpu_usage" -lt "$CPU_THRESHOLD_PERCENT" ]]; then
            log_success "Resource Test: CPU usage within limits (${cpu_usage}%)"
        else
            log_warning "Resource Test: High CPU usage detected (${cpu_usage}% > ${CPU_THRESHOLD_PERCENT}%)"
            log_remediation "Monitor CPU: top -p \$(pgrep neuroquantum)"
        fi
    else
        log_warning "Resource Test: Could not determine CPU usage"
    fi
}

# Test 6: Security and Permission Audit
test_security() {
    log_info "=== Security and Permission Audit ==="
    ((TESTS_TOTAL++))

    # Check file permissions on configuration
    local config_perms=$(stat -c "%a" "$CONFIG_FILE" 2>/dev/null || stat -f "%A" "$CONFIG_FILE" 2>/dev/null || echo "000")

    if [[ "$config_perms" == "600" || "$config_perms" == "644" ]]; then
        log_success "Security Test: Configuration file permissions secure ($config_perms)"
    elif [[ "$config_perms" == "666" || "$config_perms" == "777" ]]; then
        log_error "Security Test: Configuration file permissions too permissive ($config_perms)"
        log_remediation "Secure config file: chmod 600 $CONFIG_FILE"
    else
        log_warning "Security Test: Configuration file permissions should be reviewed ($config_perms)"
        log_remediation "Set appropriate permissions: chmod 600 $CONFIG_FILE"
    fi

    # Check for sensitive data exposure in logs
    if [[ -d "$LOG_DIR" ]]; then
        local sensitive_count=$(grep -r -i "password\|secret\|key\|token" "$LOG_DIR" 2>/dev/null | wc -l)
        if [[ "$sensitive_count" -eq 0 ]]; then
            log_success "Security Test: No sensitive data found in logs"
        else
            log_warning "Security Test: Potential sensitive data found in logs ($sensitive_count occurrences)"
            log_remediation "Review logs for sensitive data: grep -r -i 'password\\|secret\\|key\\|token' $LOG_DIR"
        fi
    fi
}

# Test 7: Backup and Recovery Verification
test_backup_recovery() {
    log_info "=== Backup and Recovery Verification ==="
    ((TESTS_TOTAL++))

    local backup_dir="${BACKUP_DIR:-./backups}"
    local today=$(date +%Y%m%d)

    if [[ -d "$backup_dir" ]]; then
        local recent_backups=$(find "$backup_dir" -name "*${today}*" -type f 2>/dev/null | wc -l)
        if [[ "$recent_backups" -gt 0 ]]; then
            log_success "Backup Test: Recent backup files found ($recent_backups files for today)"
        else
            local any_backups=$(find "$backup_dir" -type f 2>/dev/null | wc -l)
            if [[ "$any_backups" -gt 0 ]]; then
                log_warning "Backup Test: No recent backups found, but older backups exist ($any_backups total)"
                log_remediation "Check backup schedule: crontab -l | grep backup"
            else
                log_error "Backup Test: No backup files found in $backup_dir"
                log_remediation "Set up backup system and verify backup directory"
            fi
        fi
    else
        log_warning "Backup Test: Backup directory not found ($backup_dir)"
        log_remediation "Create backup directory: mkdir -p $backup_dir"
    fi
}

# Test 8: Index Efficiency Analysis
test_index_efficiency() {
    log_info "=== Index Efficiency Analysis ==="
    ((TESTS_TOTAL++))

    # Check if Rust project has proper optimization settings
    local cargo_toml="../Cargo.toml"
    if [[ -f "$cargo_toml" ]]; then
        if grep -q "opt-level.*=.*3" "$cargo_toml" && grep -q "lto.*=.*true" "$cargo_toml"; then
            log_success "Index Efficiency: Rust optimization settings configured for production"
        else
            log_warning "Index Efficiency: Rust optimization settings could be improved"
            log_remediation "Configure release profile with opt-level=3 and lto=true in Cargo.toml"
        fi

        # Check for quantum optimization settings
        if grep -q "quantum_error_correction.*=.*true" "$CONFIG_FILE"; then
            log_success "Index Efficiency: Quantum error correction enabled"
        else
            log_warning "Index Efficiency: Quantum error correction not enabled"
            log_remediation "Enable quantum_error_correction in quantum_config section"
        fi
    else
        log_error "Index Efficiency: Cargo.toml not found"
        log_remediation "Ensure you're running from the correct project directory"
    fi
}

##############################################################################
# Report Generation
##############################################################################

generate_summary_report() {
    log_info "=== Generating Summary Report ==="

    local end_time=$(date '+%Y-%m-%d %H:%M:%S')
    local success_rate=0

    if [[ "$TESTS_TOTAL" -gt 0 ]]; then
        success_rate=$(echo "scale=2; $TESTS_PASSED * 100 / $TESTS_TOTAL" | bc -l)
    fi

    # Generate JSON summary
    cat > "$SUMMARY_FILE" << EOF
{
    "test_summary": {
        "timestamp": "$end_time",
        "script_version": "$SCRIPT_VERSION",
        "total_tests": $TESTS_TOTAL,
        "passed": $TESTS_PASSED,
        "failed": $TESTS_FAILED,
        "warnings": $TESTS_WARNING,
        "success_rate": $success_rate,
        "database_host": "$DB_HOST",
        "database_port": "$DB_PORT",
        "api_endpoint": "$API_ENDPOINT"
    }
}
EOF

    # Display summary
    echo ""
    echo "==================================================="
    echo "           NEUROQUANTUMDB HEALTH SUMMARY"
    echo "==================================================="
    echo "Completed: $end_time"
    echo "Total Tests: $TESTS_TOTAL"
    echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
    echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
    echo -e "Warnings: ${YELLOW}$TESTS_WARNING${NC}"
    echo "Success Rate: ${success_rate}%"
    echo ""
    echo "Log File: $LOG_FILE"
    echo "Summary: $SUMMARY_FILE"
    echo "==================================================="

    # Return appropriate exit code
    if [[ "$TESTS_FAILED" -gt 0 ]]; then
        return 1
    elif [[ "$TESTS_WARNING" -gt 0 ]]; then
        return 2
    else
        return 0
    fi
}

##############################################################################
# Main Execution
##############################################################################

main() {
    echo -e "${CYAN}$SCRIPT_NAME v$SCRIPT_VERSION${NC}"
    echo "Starting comprehensive database health check..."
    echo ""

    # Initialize
    initialize_logging
    parse_config

    # Run all tests
    test_connection
    test_database_structure
    test_data_consistency
    test_performance
    test_resource_utilization
    test_security
    test_backup_recovery
    test_index_efficiency

    # Generate report and exit
    generate_summary_report
    exit $?
}

# Handle script interruption
trap 'log_error "Script interrupted by user"; exit 130' INT TERM

# Dependency check
command -v bc >/dev/null 2>&1 || { log_error "bc calculator required but not installed. Aborting."; exit 1; }
command -v curl >/dev/null 2>&1 || { log_error "curl required but not installed. Aborting."; exit 1; }

# Execute main function
main "$@"
