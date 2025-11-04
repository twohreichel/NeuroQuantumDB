"""Performance testing and benchmarking for NeuroQuantumDB."""

import asyncio
import statistics
import time
from typing import Any

from rich.console import Console
from rich.table import Table

from .client import NeuroQuantumClient
from .config import get_settings

console = Console()


class PerformanceTester:
    """Performance testing and benchmarking."""

    def __init__(self):
        """Initialize the performance tester."""
        self.settings = get_settings()
        self.results: dict[str, list[float]] = {}

    async def run_all_benchmarks(self) -> None:
        """Run all performance benchmarks."""
        console.print("\n[bold cyan]⚡ Performance Benchmarks[/bold cyan]\n")

        async with NeuroQuantumClient() as client:
            # Simple queries
            await self._benchmark_simple_select(client)
            await self._benchmark_filtered_select(client)
            await self._benchmark_joins(client)
            await self._benchmark_aggregations(client)

            # Quantum vs Classical comparison
            await self._benchmark_quantum_vs_classical(client)

            # Write performance
            await self._benchmark_inserts(client)

            # Display summary
            self._display_summary()

        console.print("\n[bold green]✅ Performance Tests abgeschlossen![/bold green]\n")

    async def _benchmark_simple_select(self, client: NeuroQuantumClient) -> None:
        """Benchmark simple SELECT queries."""
        console.print("\n[yellow]📊 Simple SELECT Performance[/yellow]")

        query = "SELECT * FROM customers LIMIT 100"
        times = await self._run_benchmark(client, query, "Simple SELECT", iterations=50)
        self.results["Simple SELECT"] = times

    async def _benchmark_filtered_select(self, client: NeuroQuantumClient) -> None:
        """Benchmark filtered SELECT queries."""
        console.print("\n[yellow]🔍 Filtered SELECT Performance[/yellow]")

        query = "SELECT * FROM customers WHERE age > 30 AND is_premium = 1"
        times = await self._run_benchmark(client, query, "Filtered SELECT", iterations=50)
        self.results["Filtered SELECT"] = times

    async def _benchmark_joins(self, client: NeuroQuantumClient) -> None:
        """Benchmark JOIN queries."""
        console.print("\n[yellow]🔗 JOIN Performance[/yellow]")

        query = """
        SELECT c.name, o.order_id, p.name, o.total_price
        FROM customers c
        JOIN orders o ON c.customer_id = o.customer_id
        JOIN products p ON o.product_id = p.product_id
        LIMIT 100
        """
        times = await self._run_benchmark(client, query, "3-Table JOIN", iterations=30)
        self.results["3-Table JOIN"] = times

    async def _benchmark_aggregations(self, client: NeuroQuantumClient) -> None:
        """Benchmark aggregation queries."""
        console.print("\n[yellow]📈 Aggregation Performance[/yellow]")

        query = """
        SELECT country, COUNT(*) as count, AVG(age) as avg_age
        FROM customers
        GROUP BY country
        """
        times = await self._run_benchmark(client, query, "GROUP BY Aggregation", iterations=50)
        self.results["GROUP BY Aggregation"] = times

    async def _benchmark_quantum_vs_classical(self, client: NeuroQuantumClient) -> None:
        """Compare Quantum Grover vs Classical search."""
        console.print("\n[yellow]⚛️  Quantum vs Classical Search[/yellow]")

        # Classical search
        classical_query = "SELECT * FROM customers WHERE customer_id = 500"
        classical_times = await self._run_benchmark(
            client, classical_query, "Classical Search", iterations=100
        )
        self.results["Classical Search"] = classical_times

        # Quantum search
        quantum_query = "SELECT * FROM customers WHERE customer_id = 500 USING QUANTUM GROVER"
        quantum_times = await self._run_benchmark(
            client, quantum_query, "Quantum Grover Search", iterations=100
        )
        self.results["Quantum Grover Search"] = quantum_times

        # Display comparison
        self._display_comparison(classical_times, quantum_times)

    async def _benchmark_inserts(self, client: NeuroQuantumClient) -> None:
        """Benchmark INSERT performance."""
        console.print("\n[yellow]✍️  INSERT Performance[/yellow]")

        times = []
        iterations = 20

        for i in range(iterations):
            query = f"""
            INSERT INTO customers (customer_id, name, email, country, registration_date, age, is_premium)
            VALUES ({90000 + i}, 'Test User {i}', 'test{i}@example.com', 'DE', '2024-01-01', 30, 0)
            """

            start = time.perf_counter()
            await client.execute_query(query)
            elapsed = (time.perf_counter() - start) * 1000
            times.append(elapsed)

        self.results["INSERT"] = times
        self._display_stats("INSERT", times)

        # Cleanup
        await client.execute_query("DELETE FROM customers WHERE customer_id >= 90000")

    async def _run_benchmark(
        self, client: NeuroQuantumClient, query: str, name: str, iterations: int = 50
    ) -> list[float]:
        """Run a benchmark test.

        Args:
            client: NeuroQuantumDB client
            query: Query to benchmark
            name: Benchmark name
            iterations: Number of iterations

        Returns:
            List of execution times in milliseconds
        """
        times = []

        # Warmup
        for _ in range(5):
            await client.execute_query(query)

        # Actual benchmark
        for _ in range(iterations):
            start = time.perf_counter()
            result = await client.execute_query(query)
            elapsed = (time.perf_counter() - start) * 1000

            if result.success:
                times.append(elapsed)

        self._display_stats(name, times)
        return times

    def _display_stats(self, name: str, times: list[float]) -> None:
        """Display statistics for benchmark results.

        Args:
            name: Benchmark name
            times: List of execution times
        """
        if not times:
            console.print(f"[red]Keine Daten für {name}[/red]")
            return

        mean = statistics.mean(times)
        median = statistics.median(times)
        stdev = statistics.stdev(times) if len(times) > 1 else 0
        min_time = min(times)
        max_time = max(times)

        table = Table(title=f"📊 {name}", show_header=True)
        table.add_column("Metrik", style="cyan")
        table.add_column("Wert", style="green")

        table.add_row("Durchschnitt", f"{mean:.2f} ms")
        table.add_row("Median", f"{median:.2f} ms")
        table.add_row("Std. Abweichung", f"{stdev:.2f} ms")
        table.add_row("Min", f"{min_time:.2f} ms")
        table.add_row("Max", f"{max_time:.2f} ms")
        table.add_row("Iterationen", str(len(times)))

        console.print(table)
        console.print()

    def _display_comparison(self, classical_times: list[float], quantum_times: list[float]) -> None:
        """Display comparison between classical and quantum search.

        Args:
            classical_times: Classical search times
            quantum_times: Quantum search times
        """
        classical_mean = statistics.mean(classical_times)
        quantum_mean = statistics.mean(quantum_times)

        speedup = classical_mean / quantum_mean if quantum_mean > 0 else 0

        table = Table(title="⚛️  Quantum vs Classical Comparison", show_header=True)
        table.add_column("Methode", style="cyan")
        table.add_column("Durchschnitt", style="green")
        table.add_column("Speedup", style="yellow")

        table.add_row("Classical Search", f"{classical_mean:.2f} ms", "1.0x")
        table.add_row("Quantum Grover", f"{quantum_mean:.2f} ms", f"{speedup:.2f}x")

        console.print(table)
        console.print()

        if speedup > 1:
            console.print(f"[green]✅ Quantum Search ist {speedup:.2f}x schneller![/green]\n")
        elif speedup < 1:
            console.print(f"[yellow]⚠ Classical Search ist {1/speedup:.2f}x schneller[/yellow]\n")
        else:
            console.print("[blue]ℹ️  Beide Methoden gleich schnell[/blue]\n")

    def _display_summary(self) -> None:
        """Display overall performance summary."""
        console.print("\n[bold cyan]📊 Performance Zusammenfassung[/bold cyan]\n")

        table = Table(show_header=True, header_style="bold magenta")
        table.add_column("Test", style="cyan")
        table.add_column("Durchschnitt", style="green")
        table.add_column("Median", style="yellow")
        table.add_column("Min", style="blue")
        table.add_column("Max", style="red")

        for name, times in self.results.items():
            if times:
                table.add_row(
                    name,
                    f"{statistics.mean(times):.2f} ms",
                    f"{statistics.median(times):.2f} ms",
                    f"{min(times):.2f} ms",
                    f"{max(times):.2f} ms",
                )

        console.print(table)


async def main() -> None:
    """Main function to run performance tests."""
    tester = PerformanceTester()
    await tester.run_all_benchmarks()


if __name__ == "__main__":
    asyncio.run(main())
