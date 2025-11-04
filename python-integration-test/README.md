# NeuroQuantumDB Python Integration Test Suite

Umfassendes Integrationstesting für die NeuroQuantumDB REST API.

## Features

- ✅ Vollständige REST API Integration
- ✅ Generierung realistischer, verknüpfter Testdaten via CSV
- ✅ Tests für alle Query-Typen (Standard SQL, Quantum Grover, Neuromorphic Learning)
- ✅ Performance-Benchmarks und Geschwindigkeitstests
- ✅ Async/Await mit httpx für optimale Performance
- ✅ Type-safe mit Pydantic Models
- ✅ Rich Output für bessere Lesbarkeit

## Voraussetzungen

- Python 3.12+
- Poetry
- Laufende NeuroQuantumDB Instanz (Standard: http://localhost:8080)

## Installation

```bash
cd python-integration-test
poetry install
```

## Konfiguration

Erstellen Sie eine `.env` Datei oder setzen Sie Umgebungsvariablen:

```env
NEUROQUANTUM_API_URL=http://localhost:8080
NEUROQUANTUM_API_KEY=your_api_key_here
```

## Testdaten

Die Testdaten werden automatisch generiert und enthalten:

- **Kunden** (1000 Datensätze): ID, Name, Email, Land, Registrierungsdatum
- **Produkte** (200 Datensätze): ID, Name, Kategorie, Preis, Beschreibung
- **Bestellungen** (5000 Datensätze): ID, Kunde-ID, Produkt-ID, Datum, Menge, Status
- **Biometrische Daten** (1000 Datensätze): Kunden-ID, EEG-Signale, Zeitstempel

Alle Datensätze sind sinnvoll miteinander verknüpft über Foreign Keys.

## Tests ausführen

```bash
# Alle Tests
poetry run pytest

# Nur Integration Tests
poetry run pytest -m integration

# Mit Benchmark Output
poetry run pytest -m performance --benchmark-only

# Einzelne Testmodule
poetry run pytest tests/test_basic_queries.py
poetry run pytest tests/test_quantum_queries.py
poetry run pytest tests/test_neuromorphic_queries.py

# Mit Coverage
poetry run pytest --cov=src --cov-report=html
```

## Haupttests ausführen

```bash
# Vollständiger Test-Durchlauf
poetry run python src/main.py

# Nur Daten laden
poetry run python src/data_loader.py

# Performance Tests
poetry run python src/performance_test.py
```

## Projektstruktur

```
python-integration-test/
├── pyproject.toml          # Poetry Konfiguration
├── README.md               # Diese Datei
├── .env                    # Umgebungsvariablen
├── data/                   # Generierte CSV Testdaten
│   ├── customers.csv
│   ├── products.csv
│   ├── orders.csv
│   └── biometric_data.csv
├── src/
│   ├── __init__.py
│   ├── config.py           # Konfiguration & Settings
│   ├── models.py           # Pydantic Models
│   ├── client.py           # NeuroQuantumDB REST Client
│   ├── data_generator.py   # CSV Testdaten Generator
│   ├── data_loader.py      # Daten in DB laden
│   ├── query_tester.py     # Query Tests (SQL, Quantum, Neuromorphic)
│   ├── performance_test.py # Performance Benchmarks
│   └── main.py             # Hauptprogramm
└── tests/
    ├── __init__.py
    ├── conftest.py                 # Pytest Fixtures
    ├── test_basic_queries.py       # Standard SQL Tests
    ├── test_quantum_queries.py     # Quantum Grover Tests
    ├── test_neuromorphic_queries.py # Neuromorphic Learning Tests
    └── test_performance.py         # Performance Tests
```

## Entwicklung

```bash
# Code formatieren
poetry run black src tests

# Linting
poetry run ruff check src tests

# Type checking
poetry run mypy src
```
[tool.poetry]
name = "neuroquantum-integration-test"
version = "1.0.0"
description = "Comprehensive integration testing suite for NeuroQuantumDB REST API"
authors = ["NeuroQuantum Team"]
readme = "README.md"
python = "^3.12"

[tool.poetry.dependencies]
python = "^3.12"
httpx = "^0.27.0"
pandas = "^2.2.0"
pytest = "^8.0.0"
pytest-asyncio = "^0.23.0"
pytest-benchmark = "^4.0.0"
faker = "^24.0.0"
rich = "^13.7.0"
pydantic = "^2.6.0"
pydantic-settings = "^2.2.0"
python-dotenv = "^1.0.0"
tenacity = "^8.2.3"
aiofiles = "^24.0.0"

[tool.poetry.group.dev.dependencies]
black = "^24.0.0"
ruff = "^0.3.0"
mypy = "^1.9.0"
ipython = "^8.22.0"
types-aiofiles = "^24.0.0"

[tool.black]
line-length = 100
target-version = ['py312']

[tool.ruff]
line-length = 100
target-version = "py312"

[tool.mypy]
python_version = "3.12"
strict = true
warn_return_any = true
warn_unused_configs = true
disallow_untyped_defs = true

[tool.pytest.ini_options]
asyncio_mode = "auto"
testpaths = ["tests"]
python_files = "test_*.py"
python_classes = "Test*"
python_functions = "test_*"
addopts = "-v --tb=short --strict-markers"
markers = [
    "slow: marks tests as slow (deselect with '-m \"not slow\"')",
    "integration: marks tests as integration tests",
    "performance: marks tests as performance benchmarks",
]

[build-system]
requires = ["poetry-core"]
build-backend = "poetry.core.masonry.api"

