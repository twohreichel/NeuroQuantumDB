"""Test basic SQL queries."""

import pytest
from src.client import NeuroQuantumClient


@pytest.mark.integration
class TestBasicQueries:
    """Test basic SQL query functionality."""

    async def test_select_all_customers(self, client: NeuroQuantumClient):
        """Test SELECT * from customers."""
        result = await client.execute_query("SELECT * FROM customers LIMIT 10")

        assert result.success
        assert result.rows_returned <= 10
        assert len(result.columns) > 0
        assert "customer_id" in result.columns

    async def test_count_customers(self, client: NeuroQuantumClient):
        """Test COUNT query."""
        result = await client.execute_query("SELECT COUNT(*) as total FROM customers")

        assert result.success
        assert result.rows_returned == 1
        assert len(result.rows) == 1
        assert result.rows[0][0] > 0

    async def test_where_clause(self, client: NeuroQuantumClient):
        """Test WHERE clause filtering."""
        result = await client.execute_query("SELECT * FROM customers WHERE is_premium = 1 LIMIT 10")

        assert result.success
        assert result.rows_returned >= 0

    async def test_order_by(self, client: NeuroQuantumClient):
        """Test ORDER BY clause."""
        result = await client.execute_query(
            "SELECT name, age FROM customers ORDER BY age DESC LIMIT 5"
        )

        assert result.success
        assert result.rows_returned <= 5

        # Verify ordering (ages should be descending)
        if len(result.rows) > 1:
            ages = [row[1] for row in result.rows]
            assert ages == sorted(ages, reverse=True)

    async def test_group_by(self, client: NeuroQuantumClient):
        """Test GROUP BY aggregation."""
        result = await client.execute_query(
            "SELECT country, COUNT(*) as count FROM customers GROUP BY country LIMIT 10"
        )

        assert result.success
        assert "country" in result.columns
        assert "count" in result.columns

    async def test_join_query(self, client: NeuroQuantumClient):
        """Test JOIN between tables."""
        result = await client.execute_query("""
            SELECT c.name, o.order_id, o.total_price
            FROM customers c
            JOIN orders o ON c.customer_id = o.customer_id
            LIMIT 10
        """)

        assert result.success
        assert result.rows_returned <= 10
        assert len(result.columns) == 3

    async def test_subquery(self, client: NeuroQuantumClient):
        """Test subquery."""
        result = await client.execute_query("""
            SELECT name, age FROM customers
            WHERE age > (SELECT AVG(age) FROM customers)
            LIMIT 10
        """)

        assert result.success
        assert result.rows_returned >= 0

    async def test_performance_is_acceptable(self, client: NeuroQuantumClient):
        """Test that query performance is acceptable."""
        result = await client.execute_query("SELECT * FROM customers LIMIT 100")

        assert result.success
        # Query should complete in less than 1 second
        assert result.execution_time_ms < 1000


"""Pytest configuration and fixtures."""

import pytest
import asyncio
from typing import AsyncGenerator

from src.client import NeuroQuantumClient
from src.config import get_settings


@pytest.fixture(scope="session")
def event_loop():
    """Create an event loop for async tests."""
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()


@pytest.fixture
async def client() -> AsyncGenerator[NeuroQuantumClient, None]:
    """Provide a NeuroQuantumDB client for tests."""
    async with NeuroQuantumClient() as client:
        yield client


@pytest.fixture(scope="session")
def settings():
    """Provide settings for tests."""
    return get_settings()


@pytest.fixture
async def sample_data(client: NeuroQuantumClient):
    """Create sample test data."""
    # Create test table
    await client.execute_query("""
        CREATE TABLE IF NOT EXISTS test_data (
            id INTEGER PRIMARY KEY,
            value TEXT
        )
    """)

    # Insert test data
    for i in range(10):
        await client.execute_query(f"""
            INSERT INTO test_data (id, value) VALUES ({i}, 'test_{i}')
        """)

    yield

    # Cleanup
    await client.execute_query("DROP TABLE IF EXISTS test_data")
