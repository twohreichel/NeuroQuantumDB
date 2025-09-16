# NeuroQuantumDB Test Suite

🧠⚛️🧬 **Enterprise Data Generator & Comprehensive Test Suite**

## Overview

This test suite provides comprehensive testing capabilities for NeuroQuantumDB, including:

- **Enterprise Data Generation**: 500,000+ realistic records across 6 datasets
- **Neuromorphic Query Testing**: Pattern recognition and behavioral analysis
- **Quantum Optimization**: Multi-dimensional data analysis capabilities
- **DNA Compression**: Biological encoding and storage optimization
- **Security Analysis**: Advanced threat detection and compliance testing
- **Performance Benchmarking**: Query optimization and scalability testing

## Requirements

- **Python 3.12+**
- **Poetry** (for dependency management)
- **NeuroQuantumDB** server running on `localhost:8080`

## Installation

### 1. Install Poetry (if not already installed)

```bash
curl -sSL https://install.python-poetry.org | python3 -
```

### 2. Install Dependencies

```bash
# Navigate to tests directory
cd tests

# Install all dependencies including dev, ML, quantum, and bio groups
poetry install --with dev,ml,quantum,bio

# Or install only production dependencies
poetry install --only main
```

### 3. Activate Virtual Environment

```bash
poetry shell
```

## Usage

### Generate Enterprise Data

```bash
# Generate comprehensive enterprise dataset
poetry run python test_data_generator.py --generate

# Or use the poetry script
poetry run generate-data
```

### Run All Tests

```bash
# Run complete test suite
poetry run python test_data_generator.py --test

# Or use the poetry script
poetry run run-tests
```

### Generate Data and Run Tests

```bash
# Complete workflow
poetry run python test_data_generator.py --all

# Or use the main script
poetry run neuroquantum-test --all
```

### Custom Database URL

```bash
# Test against different database instance
poetry run python test_data_generator.py --test --url http://localhost:9090
```

## Features

### 🏢 Enterprise Data Generation

- **Departments**: 25 realistic German enterprise departments
- **Employees**: 800 employees with roles, security clearances, and hierarchies
- **Documents**: 150,000 security-classified documents
- **Access Logs**: 200,000 realistic access attempts and results
- **Security Events**: 15,000 security incidents and alerts
- **Permissions**: Complex document access rights and expiration handling

### 🧠 Neuromorphic Testing

- **Pattern Recognition**: Behavioral anomaly detection
- **Learning Algorithms**: Adaptive security response
- **Neural Networks**: Synaptic plasticity simulation

### ⚛️ Quantum Computing Features

- **Superposition Queries**: Multi-dimensional data analysis
- **Entanglement**: Cross-dataset relationship analysis
- **Quantum Optimization**: Performance enhancement algorithms

### 🧬 DNA Compression

- **Biological Encoding**: DNA-based data storage
- **Compression Algorithms**: High-density information storage
- **Retrieval Optimization**: Fast biological data access

### 📊 Business Intelligence

- **Executive Dashboards**: KPI analysis and reporting
- **Security Analytics**: Threat detection and compliance
- **Performance Metrics**: Query optimization insights

## Development

### Code Quality Tools

```bash
# Format code
poetry run black .
poetry run isort .

# Lint code
poetry run flake8 .
poetry run pylint test_data_generator.py

# Type checking
poetry run mypy test_data_generator.py

# Security scanning
poetry run bandit test_data_generator.py
poetry run safety check
```

### Testing

```bash
# Run unit tests
poetry run pytest tests/ -m "unit"

# Run integration tests
poetry run pytest tests/ -m "integration"

# Run with coverage
poetry run pytest --cov=test_data_generator --cov-report=html

# Performance benchmarking
poetry run pytest tests/ --benchmark-only
```

### Pre-commit Hooks

```bash
# Install pre-commit hooks
poetry run pre-commit install

# Run hooks manually
poetry run pre-commit run --all-files
```

## Generated Data Structure

### Departments
- German enterprise structure (25 departments)
- Security levels: ÖFFENTLICH → STRENG_GEHEIM
- Budget allocation and cost centers
- Hierarchical organization

### Employees
- Realistic German names and contact information
- Role-based security clearances
- Department assignments and manager relationships
- Salary ranges and employment history

### Documents
- Security-classified content (10 document types)
- File size distribution and metadata
- Version control and retention policies
- Encryption levels based on classification

### Access Logs
- Realistic access patterns and timing
- Success/failure scenarios with reasons
- Location-based access (Office, Home, Mobile, External)
- Session tracking and data transfer metrics

### Security Events
- 12 types of security incidents
- Severity classification (LOW → CRITICAL)
- Investigation workflow and assignments
- Risk scoring and additional context data

## Configuration

### Environment Variables

```bash
# Database connection
export NEUROQUANTUM_URL="http://localhost:8080"
export NEUROQUANTUM_API_KEY="your-api-key"

# Logging level
export LOG_LEVEL="INFO"

# Test configuration
export BATCH_SIZE="1000"
export TOTAL_EMPLOYEES="800"
export TOTAL_DOCUMENTS="150000"
```

### Custom Configuration

Edit `test_data_generator.py` constants:

```python
# Data Generation Configuration
TOTAL_EMPLOYEES = 800
TOTAL_DEPARTMENTS = 25
TOTAL_DOCUMENTS = 150000
TOTAL_ACCESS_LOGS = 200000
TOTAL_SECURITY_EVENTS = 15000
BATCH_SIZE = 1000
```

## Output Files

The test suite generates several output files:

- `generated_departments.json` - Department data
- `generated_employees.json` - Employee records
- `generated_documents.json` - Document metadata
- `generated_document_permissions.json` - Access permissions
- `generated_access_logs.json` - Access attempt logs
- `generated_security_events.json` - Security incidents
- `neuroquantum_test_report_YYYYMMDD_HHMMSS.json` - Comprehensive test report

## Performance Metrics

### Expected Performance
- **Data Generation**: ~30-60 seconds for full dataset
- **Database Loading**: ~2-5 minutes depending on hardware
- **Query Execution**: <100ms for most intelligent queries
- **Memory Usage**: ~500MB-1GB during generation

### Optimization Tips
- Use SSD storage for better I/O performance
- Increase `BATCH_SIZE` for faster loading (up to 5000)
- Run tests on dedicated hardware for consistent results
- Monitor memory usage with large datasets

## Troubleshooting

### Common Issues

1. **Connection Refused**
   ```bash
   # Ensure NeuroQuantumDB is running
   curl http://localhost:8080/health
   ```

2. **Memory Issues**
   ```bash
   # Reduce dataset size temporarily
   export TOTAL_DOCUMENTS="50000"
   export TOTAL_ACCESS_LOGS="100000"
   ```

3. **Timeout Errors**
   ```bash
   # Increase timeout values in script
   # Or run tests in smaller batches
   ```

### Debug Mode

Enable detailed logging:

```python
import logging
logging.basicConfig(level=logging.DEBUG)
```

## Contributing

1. Follow PEP 8 style guidelines
2. Add type hints for all functions
3. Include comprehensive docstrings
4. Write unit tests for new features
5. Update this README for new capabilities

## License

This test suite is part of the NeuroQuantumDB project and follows the same licensing terms.
