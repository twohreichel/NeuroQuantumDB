.PHONY: install setup test clean run generate load query-test perf-test all help

help:
	@echo "NeuroQuantumDB Python Integration Test - Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  make install      - Install dependencies with Poetry"
	@echo "  make setup        - Setup environment and generate test data"
	@echo "  make generate     - Generate test data CSV files"
	@echo "  make load         - Load test data into database"
	@echo "  make query-test   - Run query tests"
	@echo "  make perf-test    - Run performance tests"
	@echo "  make test         - Run pytest test suite"
	@echo "  make run          - Run complete test suite (generate + load + test + perf)"
	@echo "  make clean        - Clean generated data and cache"
	@echo "  make format       - Format code with black"
	@echo "  make lint         - Run linting with ruff"
	@echo "  make type-check   - Run type checking with mypy"

install:
	@echo "Installing dependencies..."
	poetry install

setup: install
	@echo "Setting up environment..."
	@mkdir -p data
	@if [ ! -f .env ]; then cp .env.example .env; fi
	@echo "✓ Setup complete"

generate:
	@echo "Generating test data..."
	poetry run python src/data_generator.py

load:
	@echo "Loading data into database..."
	poetry run python src/data_loader.py

query-test:
	@echo "Running query tests..."
	poetry run python src/query_tester.py

perf-test:
	@echo "Running performance tests..."
	poetry run python src/performance_test.py

test:
	@echo "Running pytest test suite..."
	poetry run pytest -v

test-integration:
	@echo "Running integration tests..."
	poetry run pytest -v -m integration

test-performance:
	@echo "Running performance tests with pytest..."
	poetry run pytest -v -m performance

run: setup generate load query-test perf-test
	@echo ""
	@echo "✅ All tests completed!"

all: install run test

clean:
	@echo "Cleaning generated data and cache..."
	rm -rf data/*.csv
	rm -rf __pycache__
	rm -rf src/__pycache__
	rm -rf tests/__pycache__
	rm -rf .pytest_cache
	rm -rf .mypy_cache
	rm -rf htmlcov
	rm -rf .coverage
	@echo "✓ Clean complete"

format:
	@echo "Formatting code..."
	poetry run black src tests

lint:
	@echo "Running linter..."
	poetry run ruff check src tests

type-check:
	@echo "Type checking..."
	poetry run mypy src

