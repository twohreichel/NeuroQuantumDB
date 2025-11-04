"""Main entry point for NeuroQuantumDB integration test suite.

This orchestrates the complete test workflow:
1. Generate test data (CSV files)
2. Load data into NeuroQuantumDB
3. Run query tests (SQL, Quantum, Neuromorphic)
4. Run performance benchmarks
"""

import asyncio
import sys
from pathlib import Path

from rich.console import Console
from rich.panel import Panel
from rich.prompt import Confirm

# Support both direct execution and module import
if __name__ == "__main__" and __package__ is None:
    # Direct execution: add parent directory to path
    sys.path.insert(0, str(Path(__file__).parent.parent))
    from src.config import get_settings
    from src.data_generator import TestDataGenerator
    from src.data_loader import DataLoader
    from src.query_tester import QueryTester
    from src.performance_test import PerformanceTester
else:
    # Module import: use relative imports
    from .config import get_settings
    from .data_generator import TestDataGenerator
    from .data_loader import DataLoader
    from .query_tester import QueryTester
    from .performance_test import PerformanceTester

console = Console()


def print_banner() -> None:
    """Print welcome banner."""
    banner = """
    ╔═══════════════════════════════════════════════════════════╗
    ║                                                           ║
    ║        🧠 NeuroQuantumDB Integration Test Suite 🧠       ║
    ║                                                           ║
    ║  Comprehensive testing for REST API functionality        ║
    ║  • Data Generation & Loading                             ║
    ║  • Standard SQL Queries                                  ║
    ║  • Quantum Grover Search                                 ║
    ║  • Neuromorphic Learning                                 ║
    ║  • Performance Benchmarks                                ║
    ║                                                           ║
    ╚═══════════════════════════════════════════════════════════╝
    """
    console.print(banner, style="bold cyan")


def print_settings() -> None:
    """Print current settings."""
    settings = get_settings()
    console.print("\n[bold]⚙️  Konfiguration:[/bold]")
    console.print(f"  API URL:        {settings.api_base_url}")
    console.print(f"  API Key:        {'✓ gesetzt' if settings.neuroquantum_api_key else '✗ nicht gesetzt'}")
    console.print(f"  Timeout:        {settings.test_timeout}s")
    console.print(f"  Kunden:         {settings.num_customers}")
    console.print(f"  Produkte:       {settings.num_products}")
    console.print(f"  Bestellungen:   {settings.num_orders}")
    console.print(f"  Biometrische:   {settings.num_biometric_records}")
    console.print()


async def check_data_exists(data_dir: str = "data") -> bool:
    """Check if test data CSV files already exist.

    Args:
        data_dir: Directory containing CSV files

    Returns:
        True if all required CSV files exist
    """
    data_path = Path(data_dir)
    required_files = ["customers.csv", "products.csv", "orders.csv", "biometric_data.csv"]

    if not data_path.exists():
        return False

    for filename in required_files:
        if not (data_path / filename).exists():
            return False

    return True


async def generate_data_step() -> bool:
    """Step 1: Generate test data.

    Returns:
        True if successful, False otherwise
    """
    console.print("\n[bold cyan]═══ Schritt 1: Testdaten Generierung ═══[/bold cyan]\n")

    # Check if data already exists
    data_exists = await check_data_exists()

    if data_exists:
        regenerate = Confirm.ask(
            "⚠️  Testdaten existieren bereits. Neu generieren?",
            default=False
        )
        if not regenerate:
            console.print("[green]✓ Verwende existierende Testdaten[/green]")
            return True

    try:
        generator = TestDataGenerator()
        generator.generate_all()
        console.print("\n[bold green]✅ Testdaten erfolgreich generiert![/bold green]")
        return True
    except Exception as e:
        console.print(f"\n[bold red]❌ Fehler bei Datengenerierung: {e}[/bold red]")
        return False


async def load_data_step() -> bool:
    """Step 2: Load data into NeuroQuantumDB.

    Returns:
        True if successful, False otherwise
    """
    console.print("\n[bold cyan]═══ Schritt 2: Daten in NeuroQuantumDB laden ═══[/bold cyan]\n")

    # Check if data exists
    data_exists = await check_data_exists()
    if not data_exists:
        console.print("[bold red]❌ Keine Testdaten gefunden. Bitte erst generieren![/bold red]")
        return False

    try:
        loader = DataLoader()
        await loader.load_all()
        console.print("\n[bold green]✅ Daten erfolgreich geladen![/bold green]")
        return True
    except Exception as e:
        console.print(f"\n[bold red]❌ Fehler beim Laden: {e}[/bold red]")
        import traceback
        console.print(f"[dim]{traceback.format_exc()}[/dim]")
        return False


async def query_test_step() -> bool:
    """Step 3: Run query tests.

    Returns:
        True if successful, False otherwise
    """
    console.print("\n[bold cyan]═══ Schritt 3: Query Tests ═══[/bold cyan]\n")

    try:
        tester = QueryTester()
        await tester.run_all_tests()
        return True
    except Exception as e:
        console.print(f"\n[bold red]❌ Fehler bei Query Tests: {e}[/bold red]")
        import traceback
        console.print(f"[dim]{traceback.format_exc()}[/dim]")
        return False


async def performance_test_step() -> bool:
    """Step 4: Run performance benchmarks.

    Returns:
        True if successful, False otherwise
    """
    console.print("\n[bold cyan]═══ Schritt 4: Performance Benchmarks ═══[/bold cyan]\n")

    run_benchmarks = Confirm.ask(
        "⚡ Performance Benchmarks ausführen? (kann einige Minuten dauern)",
        default=True
    )

    if not run_benchmarks:
        console.print("[yellow]⊘ Performance Tests übersprungen[/yellow]")
        return True

    try:
        tester = PerformanceTester()
        await tester.run_all_benchmarks()
        return True
    except Exception as e:
        console.print(f"\n[bold red]❌ Fehler bei Performance Tests: {e}[/bold red]")
        import traceback
        console.print(f"[dim]{traceback.format_exc()}[/dim]")
        return False


async def run_full_suite() -> int:
    """Run the complete test suite.

    Returns:
        Exit code (0 for success, 1 for failure)
    """
    print_banner()
    print_settings()

    # Step 1: Generate test data
    if not await generate_data_step():
        return 1

    # Step 2: Load data
    if not await load_data_step():
        return 1

    # Step 3: Query tests
    if not await query_test_step():
        return 1

    # Step 4: Performance tests
    if not await performance_test_step():
        return 1

    # Success summary
    console.print("\n" + "═" * 60)
    console.print(
        Panel.fit(
            "[bold green]✨ Alle Tests erfolgreich abgeschlossen! ✨[/bold green]\n\n"
            "Die NeuroQuantumDB Integration ist vollständig getestet.\n"
            "Weitere Details finden Sie in den Logs oben.",
            title="🎉 Test Suite Abgeschlossen",
            border_style="green"
        )
    )
    console.print("═" * 60 + "\n")

    return 0


def main() -> int:
    """Main entry point.

    Returns:
        Exit code (0 for success, 1 for failure)
    """
    try:
        return asyncio.run(run_full_suite())
    except KeyboardInterrupt:
        console.print("\n\n[yellow]⚠️  Abgebrochen durch Benutzer[/yellow]")
        return 130
    except Exception as e:
        console.print(f"\n[bold red]❌ Unerwarteter Fehler: {e}[/bold red]")
        import traceback
        console.print(f"[dim]{traceback.format_exc()}[/dim]")
        return 1


if __name__ == "__main__":
    sys.exit(main())

