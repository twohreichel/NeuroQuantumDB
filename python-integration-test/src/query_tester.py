"""Test all query types: Standard SQL, Quantum Grover, Neuromorphic Learning."""

import asyncio
from typing import Any
from rich.console import Console
from rich.table import Table

from .client import NeuroQuantumClient

console = Console()


class QueryTester:
    """Test various query types on NeuroQuantumDB."""

    def __init__(self):
        """Initialize the query tester."""
        self.client: NeuroQuantumClient | None = None

    async def run_all_tests(self) -> None:
        """Run all query tests."""
        console.print("\n[bold cyan]🔍 Teste Query-Typen...[/bold cyan]\n")

        async with NeuroQuantumClient() as client:
            self.client = client

            # Standard SQL Tests
            await self._test_standard_sql()

            # Quantum Grover Tests
            await self._test_quantum_grover()

            # Neuromorphic Learning Tests
            await self._test_neuromorphic_learning()

            # Complex Join Tests
            await self._test_complex_joins()

            # Aggregation Tests
            await self._test_aggregations()

        console.print("\n[bold green]✅ Alle Query-Tests abgeschlossen![/bold green]\n")

    async def _test_standard_sql(self) -> None:
        """Test standard SQL queries."""
        console.print("\n[bold yellow]📊 Standard SQL Queries[/bold yellow]")
        console.print("=" * 60)

        queries = [
            ("Alle Kunden", "SELECT COUNT(*) as total FROM customers"),
            ("Premium Kunden", "SELECT COUNT(*) as total FROM customers WHERE is_premium = 1"),
            (
                "Kunden nach Land",
                "SELECT country, COUNT(*) as count FROM customers GROUP BY country ORDER BY count DESC LIMIT 5",
            ),
            (
                "Produkte nach Kategorie",
                "SELECT category, COUNT(*) as count FROM products GROUP BY category ORDER BY count DESC",
            ),
            (
                "Top 5 teuerste Produkte",
                "SELECT name, price FROM products ORDER BY price DESC LIMIT 5",
            ),
            (
                "Bestellungen nach Status",
                "SELECT status, COUNT(*) as count FROM orders GROUP BY status",
            ),
        ]

        for name, query in queries:
            await self._execute_and_display(name, query)

    async def _test_quantum_grover(self) -> None:
        """Test Quantum Grover search queries."""
        console.print("\n[bold yellow]⚛️  Quantum Grover Search[/bold yellow]")
        console.print("=" * 60)

        queries = [
            (
                "Suche spezifischen Kunden (Quantum)",
                "SELECT * FROM customers WHERE customer_id = 42 USING QUANTUM GROVER",
            ),
            (
                "Suche Produkt (Quantum)",
                "SELECT * FROM products WHERE product_id = 100 USING QUANTUM GROVER",
            ),
            (
                "Suche Premium Kunden in Land (Quantum)",
                "SELECT * FROM customers WHERE country = 'DE' AND is_premium = 1 USING QUANTUM GROVER LIMIT 10",
            ),
        ]

        for name, query in queries:
            await self._execute_and_display(name, query)

    async def _test_neuromorphic_learning(self) -> None:
        """Test Neuromorphic Learning queries."""
        console.print("\n[bold yellow]🧠 Neuromorphic Learning Queries[/bold yellow]")
        console.print("=" * 60)

        # First, create neuromorphic index
        console.print("\nErstelle Neuromorphic Index...")
        create_index = """
        CREATE NEUROMORPHIC INDEX idx_biometric_pattern 
        ON biometric_data(eeg_alpha, eeg_beta, eeg_gamma, eeg_theta)
        """
        await self._execute_query_silent(create_index)

        queries = [
            (
                "Pattern Recognition - Relaxed",
                """
                SELECT customer_id, pattern_label, eeg_alpha, eeg_beta 
                FROM biometric_data 
                WHERE pattern_label = 'relaxed' 
                LIMIT 10
                """,
            ),
            (
                "Pattern Recognition - Stressed",
                """
                SELECT customer_id, pattern_label, heart_rate, eeg_gamma
                FROM biometric_data 
                WHERE pattern_label = 'stressed' 
                ORDER BY heart_rate DESC 
                LIMIT 10
                """,
            ),
            (
                "Neuromorphic Pattern Matching",
                """
                SELECT DISTINCT pattern_label, COUNT(*) as count
                FROM biometric_data 
                GROUP BY pattern_label 
                ORDER BY count DESC
                """,
            ),
            (
                "High Alert Customers",
                """
                SELECT b.customer_id, c.name, b.pattern_label, b.heart_rate
                FROM biometric_data b
                JOIN customers c ON b.customer_id = c.customer_id
                WHERE b.pattern_label IN ('alert', 'stressed')
                ORDER BY b.heart_rate DESC
                LIMIT 15
                """,
            ),
        ]

        for name, query in queries:
            await self._execute_and_display(name, query)

    async def _test_complex_joins(self) -> None:
        """Test complex JOIN queries."""
        console.print("\n[bold yellow]🔗 Complex JOIN Queries[/bold yellow]")
        console.print("=" * 60)

        queries = [
            (
                "Kunden mit Bestellungen",
                """
                SELECT c.name, c.country, COUNT(o.order_id) as order_count
                FROM customers c
                JOIN orders o ON c.customer_id = o.customer_id
                GROUP BY c.customer_id
                ORDER BY order_count DESC
                LIMIT 10
                """,
            ),
            (
                "Top Produkte nach Umsatz",
                """
                SELECT p.name, p.category, SUM(o.total_price) as revenue
                FROM products p
                JOIN orders o ON p.product_id = o.product_id
                WHERE o.status = 'delivered'
                GROUP BY p.product_id
                ORDER BY revenue DESC
                LIMIT 10
                """,
            ),
            (
                "Kundenaktivität mit Biometrie",
                """
                SELECT c.name, COUNT(DISTINCT o.order_id) as orders,
                       COUNT(DISTINCT b.biometric_id) as biometric_records
                FROM customers c
                LEFT JOIN orders o ON c.customer_id = o.customer_id
                LEFT JOIN biometric_data b ON c.customer_id = b.customer_id
                GROUP BY c.customer_id
                HAVING orders > 5
                LIMIT 10
                """,
            ),
        ]

        for name, query in queries:
            await self._execute_and_display(name, query)

    async def _test_aggregations(self) -> None:
        """Test aggregation queries."""
        console.print("\n[bold yellow]📈 Aggregation Queries[/bold yellow]")
        console.print("=" * 60)

        queries = [
            (
                "Statistiken nach Alter",
                """
                SELECT 
                    CASE 
                        WHEN age < 30 THEN '18-29'
                        WHEN age < 50 THEN '30-49'
                        ELSE '50+'
                    END as age_group,
                    COUNT(*) as count,
                    AVG(age) as avg_age
                FROM customers
                GROUP BY age_group
                ORDER BY age_group
                """,
            ),
            (
                "Monatlicher Umsatz",
                """
                SELECT 
                    strftime('%Y-%m', order_date) as month,
                    COUNT(*) as orders,
                    SUM(total_price) as revenue
                FROM orders
                WHERE status = 'delivered'
                GROUP BY month
                ORDER BY month DESC
                LIMIT 12
                """,
            ),
            (
                "Durchschnittliche EEG-Werte pro Pattern",
                """
                SELECT 
                    pattern_label,
                    COUNT(*) as samples,
                    AVG(eeg_alpha) as avg_alpha,
                    AVG(eeg_beta) as avg_beta,
                    AVG(heart_rate) as avg_hr
                FROM biometric_data
                GROUP BY pattern_label
                ORDER BY pattern_label
                """,
            ),
        ]

        for name, query in queries:
            await self._execute_and_display(name, query)

    async def _execute_and_display(self, name: str, query: str) -> None:
        """Execute query and display results.

        Args:
            name: Query name for display
            query: SQL query string
        """
        if not self.client:
            return

        console.print(f"\n[cyan]🔍 {name}[/cyan]")

        result = await self.client.execute_query(query)

        if not result.success:
            console.print(f"[red]❌ Fehler: {result.error}[/red]")
            return

        # Display execution time
        console.print(f"⏱️  Ausführungszeit: [green]{result.execution_time_ms:.2f}ms[/green]")
        console.print(f"📊 Zeilen zurückgegeben: [green]{result.rows_returned}[/green]")

        # Display results in table
        if result.rows and result.columns:
            table = Table(show_header=True, header_style="bold magenta")

            for col in result.columns:
                table.add_column(col)

            # Show first 10 rows
            for row in result.rows[:10]:
                table.add_row(*[str(cell) for cell in row])

            if len(result.rows) > 10:
                console.print(f"\n(Zeige 10 von {len(result.rows)} Zeilen)")

            console.print(table)

    async def _execute_query_silent(self, query: str) -> None:
        """Execute query without displaying results.

        Args:
            query: SQL query string
        """
        if not self.client:
            return

        result = await self.client.execute_query(query)
        if not result.success:
            console.print(f"[yellow]⚠ Warnung: {result.error}[/yellow]")


async def main() -> None:
    """Main function to run query tests."""
    tester = QueryTester()
    await tester.run_all_tests()


if __name__ == "__main__":
    asyncio.run(main())
