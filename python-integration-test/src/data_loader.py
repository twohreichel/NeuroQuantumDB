"""Load test data from CSV files into NeuroQuantumDB."""

import asyncio
import csv
from pathlib import Path
from typing import Any

from rich.console import Console
from rich.progress import Progress, TaskID

from .client import NeuroQuantumClient
from .config import get_settings

console = Console()


class DataLoader:
    """Load test data from CSV files into NeuroQuantumDB."""

    def __init__(self, data_dir: str = "data"):
        """Initialize the data loader.

        Args:
            data_dir: Directory containing CSV files
        """
        self.data_dir = Path(data_dir)
        self.settings = get_settings()

    async def load_all(self) -> None:
        """Load all test data into the database."""
        console.print("\n[bold cyan]📦 Lade Daten in NeuroQuantumDB...[/bold cyan]\n")

        async with NeuroQuantumClient() as client:
            # Check connection
            try:
                health = await client.health_check()
                console.print(f"✓ Verbunden mit NeuroQuantumDB v{health.version}")
            except Exception as e:
                console.print(f"[bold red]❌ Verbindung fehlgeschlagen: {e}[/bold red]")
                return

            # Create tables
            await self._create_tables(client)

            # Load data in order (respecting foreign keys)
            await self._load_customers(client)
            await self._load_products(client)
            await self._load_orders(client)
            await self._load_biometric_data(client)

            # Create indexes
            await self._create_indexes(client)

        console.print("\n[bold green]✅ Alle Daten geladen![/bold green]\n")

    async def _create_tables(self, client: NeuroQuantumClient) -> None:
        """Create database tables.

        Args:
            client: NeuroQuantumDB client
        """
        console.print("\n[yellow]Erstelle Tabellen...[/yellow]")

        tables = [
            """
            CREATE TABLE IF NOT EXISTS customers (
                customer_id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL,
                country TEXT,
                registration_date TEXT,
                age INTEGER,
                is_premium INTEGER
            )
            """,
            """
            CREATE TABLE IF NOT EXISTS products (
                product_id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                category TEXT,
                price REAL NOT NULL,
                description TEXT,
                stock_quantity INTEGER
            )
            """,
            """
            CREATE TABLE IF NOT EXISTS orders (
                order_id INTEGER PRIMARY KEY,
                customer_id INTEGER NOT NULL,
                product_id INTEGER NOT NULL,
                order_date TEXT,
                quantity INTEGER,
                total_price REAL,
                status TEXT,
                FOREIGN KEY (customer_id) REFERENCES customers(customer_id),
                FOREIGN KEY (product_id) REFERENCES products(product_id)
            )
            """,
            """
            CREATE TABLE IF NOT EXISTS biometric_data (
                biometric_id INTEGER PRIMARY KEY,
                customer_id INTEGER NOT NULL,
                timestamp TEXT,
                eeg_alpha REAL,
                eeg_beta REAL,
                eeg_gamma REAL,
                eeg_theta REAL,
                heart_rate INTEGER,
                pattern_label TEXT,
                FOREIGN KEY (customer_id) REFERENCES customers(customer_id)
            )
            """,
        ]

        for table_sql in tables:
            result = await client.execute_query(table_sql)
            if not result.success:
                console.print(f"[red]Fehler beim Erstellen der Tabelle: {result.error}[/red]")

        console.print("  ✓ Tabellen erstellt\n")

    async def _load_customers(self, client: NeuroQuantumClient) -> None:
        """Load customer data from CSV.

        Args:
            client: NeuroQuantumDB client
        """
        csv_path = self.data_dir / "customers.csv"
        if not csv_path.exists():
            console.print(f"[yellow]⚠ CSV nicht gefunden: {csv_path}[/yellow]")
            return

        rows = self._read_csv(csv_path)
        console.print(f"Lade {len(rows)} Kunden...")

        with Progress() as progress:
            task = progress.add_task("[cyan]Kunden laden...", total=len(rows))

            for row in rows:
                query = f"""
                INSERT INTO customers (
                    customer_id, name, email, country, registration_date, age, is_premium
                ) VALUES (
                    {row['customer_id']},
                    '{self._escape_sql(row['name'])}',
                    '{row['email']}',
                    '{row['country']}',
                    '{row['registration_date']}',
                    {row['age']},
                    {1 if row['is_premium'] == 'True' else 0}
                )
                """
                await client.execute_query(query)
                progress.advance(task)

        console.print(f"  ✓ {len(rows)} Kunden geladen\n")

    async def _load_products(self, client: NeuroQuantumClient) -> None:
        """Load product data from CSV.

        Args:
            client: NeuroQuantumDB client
        """
        csv_path = self.data_dir / "products.csv"
        if not csv_path.exists():
            console.print(f"[yellow]⚠ CSV nicht gefunden: {csv_path}[/yellow]")
            return

        rows = self._read_csv(csv_path)
        console.print(f"Lade {len(rows)} Produkte...")

        with Progress() as progress:
            task = progress.add_task("[cyan]Produkte laden...", total=len(rows))

            for row in rows:
                query = f"""
                INSERT INTO products (
                    product_id, name, category, price, description, stock_quantity
                ) VALUES (
                    {row['product_id']},
                    '{self._escape_sql(row['name'])}',
                    '{row['category']}',
                    {row['price']},
                    '{self._escape_sql(row['description'])}',
                    {row['stock_quantity']}
                )
                """
                await client.execute_query(query)
                progress.advance(task)

        console.print(f"  ✓ {len(rows)} Produkte geladen\n")

    async def _load_orders(self, client: NeuroQuantumClient) -> None:
        """Load order data from CSV.

        Args:
            client: NeuroQuantumDB client
        """
        csv_path = self.data_dir / "orders.csv"
        if not csv_path.exists():
            console.print(f"[yellow]⚠ CSV nicht gefunden: {csv_path}[/yellow]")
            return

        rows = self._read_csv(csv_path)
        console.print(f"Lade {len(rows)} Bestellungen...")

        with Progress() as progress:
            task = progress.add_task("[cyan]Bestellungen laden...", total=len(rows))

            for row in rows:
                query = f"""
                INSERT INTO orders (
                    order_id, customer_id, product_id, order_date, 
                    quantity, total_price, status
                ) VALUES (
                    {row['order_id']},
                    {row['customer_id']},
                    {row['product_id']},
                    '{row['order_date']}',
                    {row['quantity']},
                    {row['total_price']},
                    '{row['status']}'
                )
                """
                await client.execute_query(query)
                progress.advance(task)

        console.print(f"  ✓ {len(rows)} Bestellungen geladen\n")

    async def _load_biometric_data(self, client: NeuroQuantumClient) -> None:
        """Load biometric data from CSV.

        Args:
            client: NeuroQuantumDB client
        """
        csv_path = self.data_dir / "biometric_data.csv"
        if not csv_path.exists():
            console.print(f"[yellow]⚠ CSV nicht gefunden: {csv_path}[/yellow]")
            return

        rows = self._read_csv(csv_path)
        console.print(f"Lade {len(rows)} biometrische Datensätze...")

        with Progress() as progress:
            task = progress.add_task("[cyan]Biometrische Daten laden...", total=len(rows))

            for row in rows:
                query = f"""
                INSERT INTO biometric_data (
                    biometric_id, customer_id, timestamp, eeg_alpha, eeg_beta,
                    eeg_gamma, eeg_theta, heart_rate, pattern_label
                ) VALUES (
                    {row['biometric_id']},
                    {row['customer_id']},
                    '{row['timestamp']}',
                    {row['eeg_alpha']},
                    {row['eeg_beta']},
                    {row['eeg_gamma']},
                    {row['eeg_theta']},
                    {row['heart_rate']},
                    '{row['pattern_label']}'
                )
                """
                await client.execute_query(query)
                progress.advance(task)

        console.print(f"  ✓ {len(rows)} biometrische Datensätze geladen\n")

    async def _create_indexes(self, client: NeuroQuantumClient) -> None:
        """Create database indexes for performance.

        Args:
            client: NeuroQuantumDB client
        """
        console.print("\n[yellow]Erstelle Indizes...[/yellow]")

        indexes = [
            "CREATE INDEX IF NOT EXISTS idx_customers_country ON customers(country)",
            "CREATE INDEX IF NOT EXISTS idx_customers_age ON customers(age)",
            "CREATE INDEX IF NOT EXISTS idx_products_category ON products(category)",
            "CREATE INDEX IF NOT EXISTS idx_orders_customer ON orders(customer_id)",
            "CREATE INDEX IF NOT EXISTS idx_orders_product ON orders(product_id)",
            "CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status)",
            "CREATE INDEX IF NOT EXISTS idx_biometric_customer ON biometric_data(customer_id)",
            "CREATE INDEX IF NOT EXISTS idx_biometric_pattern ON biometric_data(pattern_label)",
        ]

        for index_sql in indexes:
            await client.execute_query(index_sql)

        console.print("  ✓ Indizes erstellt\n")

    def _read_csv(self, filepath: Path) -> list[dict[str, Any]]:
        """Read CSV file into list of dictionaries.

        Args:
            filepath: Path to CSV file

        Returns:
            List of row dictionaries
        """
        with open(filepath, "r", encoding="utf-8") as f:
            return list(csv.DictReader(f))

    @staticmethod
    def _escape_sql(value: str) -> str:
        """Escape SQL string value.

        Args:
            value: String to escape

        Returns:
            Escaped string
        """
        return value.replace("'", "''")


async def main() -> None:
    """Main function to load test data."""
    loader = DataLoader()
    await loader.load_all()


if __name__ == "__main__":
    asyncio.run(main())
