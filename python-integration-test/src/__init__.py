"""Package initialization."""

__version__ = "1.0.0"
"""Main entry point for NeuroQuantumDB integration tests."""

import asyncio
import sys
from rich.console import Console
from rich.panel import Panel

from .config import get_settings
from .data_generator import TestDataGenerator
from .data_loader import DataLoader
from .query_tester import QueryTester
from .performance_test import PerformanceTester

console = Console()


async def main() -> None:
    """Main function to run all integration tests."""
    console.print(
        Panel.fit(
            "[bold cyan]NeuroQuantumDB Integration Test Suite[/bold cyan]\n"
            "Comprehensive testing of REST API, Quantum & Neuromorphic features",
            border_style="cyan",
        )
    )

    settings = get_settings()

    # Display configuration
    console.print("\n[bold]Konfiguration:[/bold]")
    console.print(f"  API URL: {settings.neuroquantum_api_url}")
    console.print(f"  Timeout: {settings.test_timeout}s")
    console.print(f"  Kunden: {settings.num_customers}")
    console.print(f"  Produkte: {settings.num_products}")
    console.print(f"  Bestellungen: {settings.num_orders}")
    console.print(f"  Biometrische Daten: {settings.num_biometric_records}\n")

    try:
        # Step 1: Generate test data
        console.print("\n[bold]Schritt 1/4: Testdaten generieren[/bold]")
        generator = TestDataGenerator()
        generator.generate_all()

        # Step 2: Load data into database
        console.print("\n[bold]Schritt 2/4: Daten in Datenbank laden[/bold]")
        loader = DataLoader()
        await loader.load_all()

        # Step 3: Test all query types
        console.print("\n[bold]Schritt 3/4: Query-Typen testen[/bold]")
        tester = QueryTester()
        await tester.run_all_tests()

        # Step 4: Performance benchmarks
        console.print("\n[bold]Schritt 4/4: Performance-Tests[/bold]")
        perf_tester = PerformanceTester()
        await perf_tester.run_all_benchmarks()

        # Final summary
        console.print("\n" + "=" * 60)
        console.print(
            Panel.fit(
                "[bold green]✅ Alle Tests erfolgreich abgeschlossen![/bold green]\n\n"
                "Die NeuroQuantumDB REST API wurde umfassend getestet:\n"
                "  ✓ Standard SQL Queries\n"
                "  ✓ Quantum Grover Search\n"
                "  ✓ Neuromorphic Learning\n"
                "  ✓ Performance Benchmarks\n",
                border_style="green",
                title="Test Summary",
            )
        )

    except KeyboardInterrupt:
        console.print("\n\n[yellow]⚠️  Tests unterbrochen durch Benutzer[/yellow]")
        sys.exit(1)
    except Exception as e:
        console.print(f"\n\n[bold red]❌ Fehler während der Tests: {e}[/bold red]")
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
