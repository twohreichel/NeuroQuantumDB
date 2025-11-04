"""Generate realistic test data with interconnected relationships."""

import csv
import random
from datetime import datetime, timedelta
from decimal import Decimal
from pathlib import Path
from typing import Any

from faker import Faker
from rich.console import Console
from rich.progress import Progress

from .config import get_settings

console = Console()
fake = Faker(["de_DE", "en_US", "fr_FR", "es_ES"])


class TestDataGenerator:
    """Generate realistic, interconnected test data for NeuroQuantumDB."""

    def __init__(self, output_dir: str = "data"):
        """Initialize the data generator.

        Args:
            output_dir: Directory to save CSV files
        """
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(exist_ok=True)
        self.settings = get_settings()

    def generate_all(self) -> None:
        """Generate all test data CSV files."""
        console.print("\n[bold cyan]🎲 Generiere Testdaten...[/bold cyan]\n")

        with Progress() as progress:
            task = progress.add_task("[cyan]Generiere Daten...", total=4)

            # Generate in order to maintain referential integrity
            customers = self.generate_customers()
            progress.advance(task)

            products = self.generate_products()
            progress.advance(task)

            orders = self.generate_orders(customers, products)
            progress.advance(task)

            biometric = self.generate_biometric_data(customers)
            progress.advance(task)

        console.print("\n[bold green]✅ Alle Testdaten generiert![/bold green]\n")

    def generate_customers(self) -> list[dict[str, Any]]:
        """Generate customer data.

        Returns:
            List of customer dictionaries
        """
        customers = []

        for i in range(1, self.settings.num_customers + 1):
            registration_date = fake.date_between(start_date="-3y", end_date="today")

            customer = {
                "customer_id": i,
                "name": fake.name(),
                "email": fake.email(),
                "country": fake.country_code(),
                "registration_date": registration_date.isoformat(),
                "age": random.randint(18, 80),
                "is_premium": random.choice([True, False]),
            }
            customers.append(customer)

        # Save to CSV
        csv_path = self.output_dir / "customers.csv"
        self._write_csv(csv_path, customers)
        console.print(f"  ✓ Kunden: {len(customers)} Datensätze → {csv_path}")

        return customers

    def generate_products(self) -> list[dict[str, Any]]:
        """Generate product data.

        Returns:
            List of product dictionaries
        """
        categories = [
            "Elektronik",
            "Kleidung",
            "Bücher",
            "Haushalt",
            "Sport",
            "Garten",
            "Spielzeug",
            "Lebensmittel",
        ]

        products = []

        for i in range(1, self.settings.num_products + 1):
            category = random.choice(categories)

            product = {
                "product_id": i,
                "name": fake.catch_phrase(),
                "category": category,
                "price": round(Decimal(random.uniform(9.99, 999.99)), 2),
                "description": fake.text(max_nb_chars=200),
                "stock_quantity": random.randint(0, 1000),
            }
            products.append(product)

        # Save to CSV
        csv_path = self.output_dir / "products.csv"
        self._write_csv(csv_path, products)
        console.print(f"  ✓ Produkte: {len(products)} Datensätze → {csv_path}")

        return products

    def generate_orders(
        self, customers: list[dict[str, Any]], products: list[dict[str, Any]]
    ) -> list[dict[str, Any]]:
        """Generate order data with foreign key relationships.

        Args:
            customers: List of customer records
            products: List of product records

        Returns:
            List of order dictionaries
        """
        statuses = ["pending", "processing", "shipped", "delivered", "cancelled"]
        orders = []

        for i in range(1, self.settings.num_orders + 1):
            customer = random.choice(customers)
            product = random.choice(products)
            quantity = random.randint(1, 10)

            order_date = fake.date_time_between(
                start_date=datetime.fromisoformat(customer["registration_date"]), end_date="now"
            )

            order = {
                "order_id": i,
                "customer_id": customer["customer_id"],
                "product_id": product["product_id"],
                "order_date": order_date.isoformat(),
                "quantity": quantity,
                "total_price": round(Decimal(product["price"]) * quantity, 2),
                "status": random.choice(statuses),
            }
            orders.append(order)

        # Save to CSV
        csv_path = self.output_dir / "orders.csv"
        self._write_csv(csv_path, orders)
        console.print(f"  ✓ Bestellungen: {len(orders)} Datensätze → {csv_path}")

        return orders

    def generate_biometric_data(self, customers: list[dict[str, Any]]) -> list[dict[str, Any]]:
        """Generate biometric data for neuromorphic learning tests.

        Args:
            customers: List of customer records

        Returns:
            List of biometric data dictionaries
        """
        patterns = ["relaxed", "focused", "stressed", "alert", "drowsy"]
        biometric_records = []

        for i in range(1, self.settings.num_biometric_records + 1):
            customer = random.choice(customers)
            pattern = random.choice(patterns)

            # Generate realistic EEG patterns based on state
            base_values = {
                "relaxed": {"alpha": 0.8, "beta": 0.2, "gamma": 0.1, "theta": 0.7, "hr": 65},
                "focused": {"alpha": 0.4, "beta": 0.9, "gamma": 0.6, "theta": 0.3, "hr": 75},
                "stressed": {"alpha": 0.3, "beta": 0.9, "gamma": 0.8, "theta": 0.4, "hr": 90},
                "alert": {"alpha": 0.5, "beta": 0.7, "gamma": 0.5, "theta": 0.4, "hr": 80},
                "drowsy": {"alpha": 0.6, "beta": 0.3, "gamma": 0.2, "theta": 0.9, "hr": 60},
            }

            base = base_values[pattern]

            record = {
                "biometric_id": i,
                "customer_id": customer["customer_id"],
                "timestamp": fake.date_time_between(start_date="-1y", end_date="now").isoformat(),
                "eeg_alpha": round(base["alpha"] + random.uniform(-0.1, 0.1), 4),
                "eeg_beta": round(base["beta"] + random.uniform(-0.1, 0.1), 4),
                "eeg_gamma": round(base["gamma"] + random.uniform(-0.1, 0.1), 4),
                "eeg_theta": round(base["theta"] + random.uniform(-0.1, 0.1), 4),
                "heart_rate": base["hr"] + random.randint(-5, 5),
                "pattern_label": pattern,
            }
            biometric_records.append(record)

        # Save to CSV
        csv_path = self.output_dir / "biometric_data.csv"
        self._write_csv(csv_path, biometric_records)
        console.print(f"  ✓ Biometrische Daten: {len(biometric_records)} Datensätze → {csv_path}")

        return biometric_records

    def _write_csv(self, filepath: Path, data: list[dict[str, Any]]) -> None:
        """Write data to CSV file.

        Args:
            filepath: Path to CSV file
            data: List of dictionaries to write
        """
        if not data:
            return

        with open(filepath, "w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, fieldnames=data[0].keys())
            writer.writeheader()
            writer.writerows(data)


def main() -> None:
    """Main function to generate test data."""
    generator = TestDataGenerator()
    generator.generate_all()


if __name__ == "__main__":
    main()
