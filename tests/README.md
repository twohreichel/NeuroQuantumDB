# NeuroQuantumDB Database Testing Framework

## Overview
Enterprise-grade database health check and monitoring system for NeuroQuantumDB, designed for production environments with comprehensive error detection, performance monitoring, and automated remediation guidance.

## Features
- **Comprehensive Health Checks**: 8 critical test categories covering all aspects of database operations
- **Structured Logging**: Timestamped, categorized logs with severity levels and color coding
- **Performance Monitoring**: Real-time metrics with configurable thresholds
- **Security Auditing**: Permission checks and sensitive data exposure detection
- **Automated Remediation**: Specific fix suggestions for each detected issue
- **JSON Reporting**: Machine-readable summary reports for integration with monitoring systems
- **Cross-Platform**: Compatible with Linux, Unix, and macOS systems

## Quick Start

### Prerequisites
```bash
# Install required dependencies
# macOS:
brew install bc curl

# Ubuntu/Debian:
sudo apt-get install bc curl

# RHEL/CentOS:
sudo yum install bc curl
```

### Basic Usage
```bash
# Navigate to tests directory
cd /path/to/NeuroQuantumDB/tests

# Source configuration (optional)
source test_config.env

# Run complete health check
./db_health_check.sh

# Run with custom config
CONFIG_FILE="../config/prod.toml" ./db_health_check.sh
```

## Test Categories

### 1. Connection and Authentication Test
- **Purpose**: Validates API endpoint accessibility and response time
- **Checks**: HTTP connectivity, response latency, timeout handling
- **Thresholds**: Connection timeout < 5000ms
- **Remediation**: Network connectivity, service status verification

### 2. Database Structure Integrity Test
- **Purpose**: Verifies binary existence, dependencies, and executability
- **Checks**: Binary compilation, library dependencies, file permissions
- **Remediation**: Build instructions, dependency installation

### 3. Data Consistency and Validation Test
- **Purpose**: Validates neuromorphic and quantum configuration parameters
- **Checks**: Parameter ranges, configuration syntax, logical consistency
- **Remediation**: Configuration corrections, parameter optimization

### 4. Performance Benchmarking Test
- **Purpose**: Measures API response times and identifies bottlenecks
- **Checks**: Average response time over multiple requests
- **Thresholds**: Query timeout < 1000ms, slow query < 2000ms
- **Remediation**: Performance optimization, resource scaling

### 5. Resource Utilization Test
- **Purpose**: Monitors system memory and CPU usage
- **Checks**: Memory consumption, CPU load, resource limits
- **Thresholds**: Memory < 1024MB, CPU < 80%
- **Remediation**: Resource optimization, scaling recommendations

### 6. Security and Permission Audit
- **Purpose**: Validates file permissions and checks for data exposure
- **Checks**: Configuration file permissions, log security, access controls
- **Remediation**: Permission fixes, security hardening

### 7. Backup and Recovery Verification
- **Purpose**: Ensures backup systems are functioning correctly
- **Checks**: Backup file existence, backup schedule, recovery procedures
- **Remediation**: Backup system setup, schedule configuration

### 8. Index Efficiency Analysis
- **Purpose**: Validates optimization settings and quantum configurations
- **Checks**: Rust compilation settings, quantum error correction
- **Remediation**: Optimization configuration, performance tuning

## Configuration

### Environment Variables
```bash
# Core Settings
CONFIG_FILE="../config/dev.toml"    # Path to NeuroQuantumDB config
LOG_DIR="./logs"                    # Log output directory
BACKUP_DIR="./backups"              # Backup verification directory

# Performance Thresholds
CONNECTION_TIMEOUT_MS=5000          # Maximum connection time
QUERY_TIMEOUT_MS=1000              # Query response threshold
SLOW_QUERY_THRESHOLD_MS=2000       # Slow query warning threshold
MEMORY_THRESHOLD_MB=1024           # Memory usage warning
CPU_THRESHOLD_PERCENT=80           # CPU usage warning
```

### Custom Configuration
```bash
# Create custom environment file
cp test_config.env my_config.env
# Edit thresholds and paths
vim my_config.env
# Use custom config
source my_config.env && ./db_health_check.sh
```

## Output Format

### Console Output
```
[2025-09-19 12:17:01] [INFO] === DATABASE HEALTH CHECK STARTED ===
[2025-09-19 12:17:02] [SUCCESS] ✓ Connection Test: API endpoint reachable (Response: 23ms)
[2025-09-19 12:17:03] [WARNING] ⚠ Performance Test: Slow query detected (Query time: 2.3s > threshold: 1s)
[2025-09-19 12:17:03] [REMEDIATION] → Monitor system resources and optimize API endpoints
[2025-09-19 12:17:04] [ERROR] ✗ Backup Test: Backup directory not found
[2025-09-19 12:17:04] [REMEDIATION] → Create backup directory: mkdir -p ./backups
```

### JSON Summary Report
```json
{
    "test_summary": {
        "timestamp": "2025-09-19 12:17:05",
        "script_version": "1.0.0",
        "total_tests": 8,
        "passed": 6,
        "failed": 1,
        "warnings": 1,
        "success_rate": 75.00,
        "database_host": "localhost",
        "database_port": "8080",
        "api_endpoint": "http://localhost:8080"
    }
}
```

## Integration

### CI/CD Pipeline Integration
```yaml
# GitHub Actions example
- name: Database Health Check
  run: |
    cd tests
    ./db_health_check.sh
    if [ $? -eq 1 ]; then
      echo "Critical failures detected"
      exit 1
    elif [ $? -eq 2 ]; then
      echo "Warnings detected, proceeding with caution"
    fi
```

### Monitoring System Integration
```bash
# Prometheus metrics export (future enhancement)
./db_health_check.sh && curl -X POST \
  http://prometheus-gateway:9091/metrics/job/neuroquantum-health \
  --data-binary @logs/health_summary_$(date +%Y%m%d).json
```

### Cron Job Setup
```bash
# Daily health check at 2 AM
0 2 * * * /path/to/NeuroQuantumDB/tests/db_health_check.sh >> /var/log/neuroquantum-health.log 2>&1

# Hourly quick check during business hours
0 9-17 * * 1-5 /path/to/NeuroQuantumDB/tests/db_health_check.sh --quick
```

## Exit Codes
- **0**: All tests passed successfully
- **1**: One or more critical failures detected
- **2**: Warnings detected but no critical failures
- **130**: Script interrupted by user

## Log Management
- Automatic log rotation (removes files older than 7 days)
- Structured logging with timestamps and severity levels
- Color-coded console output for better readability
- JSON summary reports for automated processing

## Troubleshooting

### Common Issues

#### "Configuration file not found"
```bash
# Verify config path
ls -la ../config/dev.toml
# Set correct path
export CONFIG_FILE="/full/path/to/config.toml"
```

#### "bc calculator required but not installed"
```bash
# Install bc calculator
sudo apt-get install bc  # Ubuntu/Debian
brew install bc          # macOS
sudo yum install bc      # RHEL/CentOS
```

#### "API endpoint unreachable"
```bash
# Check if service is running
ps aux | grep neuroquantum
# Check port availability
netstat -tlnp | grep 8080
# Test manual connection
curl -v http://localhost:8080/health
```

### Performance Optimization
- Adjust thresholds based on your environment
- Monitor resource usage during peak loads
- Configure appropriate timeouts for your network conditions
- Scale resources based on warning recommendations

## Security Considerations
- Store configuration files with restricted permissions (600)
- Avoid logging sensitive information
- Regularly audit file permissions and access controls
- Use environment variables for sensitive configuration

## Support
For issues and feature requests, please refer to the NeuroQuantumDB project documentation or contact the development team.
