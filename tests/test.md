## Database Testing Shell Script Prompt

### **R - Role (Rolle)**
You are a Senior DevOps Engineer and Database Specialist with 15+ years of experience in enterprise database management, specializing in automated testing frameworks, monitoring systems, and comprehensive error diagnosis. You have expertise in shell scripting, database performance optimization, and production system reliability.

### **E - Expected Result (Ergebnis)**
Create a production-ready shell script that meets enterprise-grade standards:
- Comprehensive database health check and stress testing functionality
- Structured, readable log file output with detailed error classification
- Clear problem identification with specific remediation steps
- Modular test structure for easy maintenance and extension
- Error handling with proper exit codes and recovery mechanisms
- Performance metrics and benchmark reporting
- Integration-ready for CI/CD pipelines and monitoring systems

### **I - Information Detail (Inhalt detaillieren)**
**Technical Specifications:**
- Programming language: Bash shell script (compatible with Linux/Unix/MAC systems)
- Architecture: Modular testing framework with separate test categories
- Logging: Structured output with timestamps, severity levels, and categorization
- Performance: Measure query response times, connection pooling, and resource usage
- Security: Include connection security, user permissions, and data integrity tests
- Output format: Human-readable log files with color coding and summary sections
- Error handling: Graceful failure handling with detailed diagnostic information

**Test Categories to Include:**
- Connection and authentication tests
- Database structure integrity checks
- Data consistency and validation tests
- Performance benchmarking (CRUD operations)
- Index efficiency analysis
- Backup and recovery verification
- Security and permission audits
- Resource utilization monitoring

### **Z - Target Audience (Zielgruppe)**
Target: Database administrators and DevOps engineers in enterprise environment
Context: Production database systems requiring regular health monitoring
Explanation: Include technical reasoning for each test and clear remediation steps
Team considerations: Script should be maintainable and extensible by multiple team members

### **B - Examples (Beispiele)**
**Log Output Structure:**
```
[2025-09-19 12:17:01] [INFO] === DATABASE HEALTH CHECK STARTED ===
[2025-09-19 12:17:02] [SUCCESS] ✓ Connection Test: MySQL connection established (Response: 23ms)
[2025-09-19 12:17:03] [WARNING] ⚠ Performance Test: Slow query detected (Query time: 2.3s > threshold: 1s)
[2025-09-19 12:17:03] [REMEDIATION] → Optimize query: ANALYZE TABLE users; or add index on column 'email'
[2025-09-19 12:17:04] [ERROR] ✗ Backup Test: Backup file not found at /backup/db_backup_20250919.sql
[2025-09-19 12:17:04] [REMEDIATION] → Check backup cron job: 'crontab -l | grep backup' and verify backup script permissions
```

**Test Function Template:**
```bash
test_connection() {
    local start_time=$(date +%s.%N)
    local result=$(mysql -h $DB_HOST -u $DB_USER -p$DB_PASS -e "SELECT 1;" 2>&1)
    local end_time=$(date +%s.%N)
    local duration=$(echo "$end_time - $start_time" | bc)
    
    if [ $? -eq 0 ]; then
        log_success "Connection Test: Database connection established (Response: ${duration}ms)"
    else
        log_error "Connection Test: Failed to connect to database"
        log_remediation "Check database credentials and network connectivity: telnet $DB_HOST $DB_PORT"
    fi
}
```

### **A - Assignment Clarification (Auftragsklärung)**
**Pre-Implementation Analysis:**
1. Which specific database system(s) should the script support primarily?
2. What are the typical database performance benchmarks for your system?
3. Should the script integrate with existing monitoring tools (Nagios, Zabbix, etc.)?
4. What specific database schema elements need validation?
5. Are there specific backup and recovery procedures to test?
6. What are the preferred log file locations and retention policies?

**Assumptions for Implementation:**
- Script will run on Linux/Unix systems with standard shell utilities
- Database credentials will be provided via environment variables or config file
- Log files should be rotated and archived automatically
- Tests should be non-destructive to production data
- Script should support both manual execution and automated scheduling

### **R - Revision Planning (Revision)**
**Comprehensive Review Process:**
- Shell script best practices compliance (ShellCheck validation)
- Security assessment for credential handling and SQL injection prevention
- Performance testing with various database sizes and loads
- Error handling validation for all failure scenarios
- Log format consistency and parsing compatibility
- Documentation completeness for maintenance and troubleshooting
- Integration testing with monitoring and alerting systems
- Cross-platform compatibility verification

**Quality Requirements:**
- All database operations must be read-only unless explicitly testing write operations
- Comprehensive error logging with severity classification
- Modular design allowing individual test execution
- Configuration file support for different environments
- Automated summary report generation
- Integration hooks for external monitoring systems