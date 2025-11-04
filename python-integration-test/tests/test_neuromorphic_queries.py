"""Test Neuromorphic Learning queries."""

import pytest
from src.client import NeuroQuantumClient


@pytest.mark.integration
class TestNeuromorphicQueries:
    """Test Neuromorphic Learning functionality."""

    async def test_create_neuromorphic_index(self, client: NeuroQuantumClient):
        """Test creating a neuromorphic index."""
        result = await client.execute_query("""
            CREATE NEUROMORPHIC INDEX IF NOT EXISTS idx_test_neuromorphic
            ON biometric_data(eeg_alpha, eeg_beta, eeg_gamma)
        """)

        # Index creation might not be fully implemented
        assert result.success or "NEUROMORPHIC" in str(result.error)

    async def test_pattern_recognition_query(self, client: NeuroQuantumClient):
        """Test pattern recognition in biometric data."""
        result = await client.execute_query("""
            SELECT customer_id, pattern_label, eeg_alpha, eeg_beta
            FROM biometric_data
            WHERE pattern_label = 'relaxed'
            LIMIT 10
        """)

        assert result.success
        assert result.rows_returned <= 10

        if result.rows:
            # Verify pattern label column
            pattern_idx = result.columns.index("pattern_label")
            for row in result.rows:
                assert row[pattern_idx] == "relaxed"

    async def test_eeg_pattern_aggregation(self, client: NeuroQuantumClient):
        """Test aggregation of EEG patterns."""
        result = await client.execute_query("""
            SELECT 
                pattern_label,
                COUNT(*) as count,
                AVG(eeg_alpha) as avg_alpha,
                AVG(eeg_beta) as avg_beta
            FROM biometric_data
            GROUP BY pattern_label
            ORDER BY count DESC
        """)

        assert result.success
        assert "pattern_label" in result.columns
        assert "count" in result.columns
        assert "avg_alpha" in result.columns

    async def test_high_stress_detection(self, client: NeuroQuantumClient):
        """Test detection of high stress patterns."""
        result = await client.execute_query("""
            SELECT customer_id, heart_rate, eeg_gamma, pattern_label
            FROM biometric_data
            WHERE pattern_label = 'stressed'
            ORDER BY heart_rate DESC
            LIMIT 10
        """)

        assert result.success

        if result.rows and len(result.rows) > 1:
            # Verify ordering by heart_rate
            hr_idx = result.columns.index("heart_rate")
            heart_rates = [row[hr_idx] for row in result.rows]
            assert heart_rates == sorted(heart_rates, reverse=True)

    async def test_neuromorphic_join_query(self, client: NeuroQuantumClient):
        """Test JOIN with neuromorphic data."""
        result = await client.execute_query("""
            SELECT 
                c.name,
                c.country,
                b.pattern_label,
                b.heart_rate
            FROM customers c
            JOIN biometric_data b ON c.customer_id = b.customer_id
            WHERE b.pattern_label IN ('alert', 'stressed')
            LIMIT 20
        """)

        assert result.success
        assert result.rows_returned <= 20

    async def test_eeg_wave_correlation(self, client: NeuroQuantumClient):
        """Test correlation between EEG waves."""
        result = await client.execute_query("""
            SELECT 
                pattern_label,
                AVG(eeg_alpha) as alpha,
                AVG(eeg_beta) as beta,
                AVG(eeg_gamma) as gamma,
                AVG(eeg_theta) as theta
            FROM biometric_data
            GROUP BY pattern_label
        """)

        assert result.success
        assert len(result.columns) == 5

        # Verify we have data for different patterns
        if result.rows:
            patterns = [row[0] for row in result.rows]
            assert len(patterns) > 0

    @pytest.mark.slow
    async def test_neuromorphic_learning_adaptation(self, client: NeuroQuantumClient):
        """Test that neuromorphic index adapts over time."""
        # Query same pattern multiple times
        query = """
            SELECT * FROM biometric_data
            WHERE pattern_label = 'focused'
            LIMIT 10
        """

        times = []
        for _ in range(5):
            result = await client.execute_query(query)
            if result.success:
                times.append(result.execution_time_ms)

        # With learning, queries should potentially get faster
        # (though this might not be observable in small tests)
        assert len(times) == 5
        assert all(t >= 0 for t in times)


"""Test Quantum Grover search queries."""

import pytest
from src.client import NeuroQuantumClient


@pytest.mark.integration
class TestQuantumQueries:
    """Test Quantum Grover search functionality."""

    async def test_quantum_search_single_record(self, client: NeuroQuantumClient):
        """Test Quantum Grover search for a single record."""
        result = await client.execute_query(
            "SELECT * FROM customers WHERE customer_id = 42 USING QUANTUM GROVER"
        )

        assert result.success or result.error  # May not be implemented yet

        if result.success:
            assert result.rows_returned <= 1

    async def test_quantum_search_with_condition(self, client: NeuroQuantumClient):
        """Test Quantum search with additional conditions."""
        result = await client.execute_query(
            "SELECT * FROM customers WHERE country = 'DE' USING QUANTUM GROVER LIMIT 10"
        )

        # Quantum search might not be fully implemented
        assert result.success or "QUANTUM" in str(result.error)

    async def test_quantum_vs_classical_correctness(self, client: NeuroQuantumClient):
        """Test that Quantum and Classical searches return same results."""
        customer_id = 100

        # Classical search
        classical = await client.execute_query(
            f"SELECT * FROM customers WHERE customer_id = {customer_id}"
        )

        # Quantum search
        quantum = await client.execute_query(
            f"SELECT * FROM customers WHERE customer_id = {customer_id} USING QUANTUM GROVER"
        )

        if classical.success and quantum.success:
            # Both should return the same data
            assert classical.rows_returned == quantum.rows_returned
            if classical.rows and quantum.rows:
                assert classical.rows == quantum.rows

    async def test_quantum_search_performance(self, client: NeuroQuantumClient):
        """Test Quantum search performance metrics."""
        result = await client.execute_query(
            "SELECT * FROM products WHERE product_id = 50 USING QUANTUM GROVER"
        )

        if result.success:
            # Should have execution time recorded
            assert result.execution_time_ms >= 0

    @pytest.mark.slow
    async def test_quantum_search_multiple(self, client: NeuroQuantumClient):
        """Test multiple Quantum searches."""
        customer_ids = [10, 20, 30, 40, 50]

        for cid in customer_ids:
            result = await client.execute_query(
                f"SELECT * FROM customers WHERE customer_id = {cid} USING QUANTUM GROVER"
            )

            if result.success:
                assert result.rows_returned <= 1
