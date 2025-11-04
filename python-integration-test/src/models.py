"""Pydantic models for NeuroQuantumDB API responses and data structures."""

from datetime import datetime
from decimal import Decimal
from typing import Any, Optional
from pydantic import BaseModel, Field, ConfigDict


class QueryRequest(BaseModel):
    """QSQL Query request."""

    query: str


class QueryResponse(BaseModel):
    """Query execution response."""

    success: bool
    columns: list[str] = Field(default_factory=list)
    rows: list[list[Any]] = Field(default_factory=list)
    rows_returned: int = 0
    execution_time_ms: float = 0.0
    error: Optional[str] = None


class HealthResponse(BaseModel):
    """Health check response."""

    status: str
    version: str
    uptime_seconds: int


class APIKeyRequest(BaseModel):
    """API key creation request."""

    name: str
    permissions: list[str]
    expires_in_hours: int


class APIKeyResponse(BaseModel):
    """API key creation response."""

    api_key: str
    key_id: str
    expires_at: str


# Domain Models for Test Data


class Customer(BaseModel):
    """Customer domain model."""

    model_config = ConfigDict(from_attributes=True)

    customer_id: int
    name: str
    email: str
    country: str
    registration_date: str
    age: int
    is_premium: bool


class Product(BaseModel):
    """Product domain model."""

    model_config = ConfigDict(from_attributes=True)

    product_id: int
    name: str
    category: str
    price: Decimal
    description: str
    stock_quantity: int


class Order(BaseModel):
    """Order domain model."""

    model_config = ConfigDict(from_attributes=True)

    order_id: int
    customer_id: int
    product_id: int
    order_date: str
    quantity: int
    total_price: Decimal
    status: str


class BiometricData(BaseModel):
    """Biometric data model for neuromorphic tests."""

    model_config = ConfigDict(from_attributes=True)

    biometric_id: int
    customer_id: int
    timestamp: str
    eeg_alpha: float
    eeg_beta: float
    eeg_gamma: float
    eeg_theta: float
    heart_rate: int
    pattern_label: str


"""Configuration and settings for NeuroQuantumDB integration tests."""

from pydantic_settings import BaseSettings, SettingsConfigDict
from functools import lru_cache


class Settings(BaseSettings):
    """Application settings loaded from environment or .env file."""

    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
    )

    # API Configuration
    neuroquantum_api_url: str = "http://localhost:8080"
    neuroquantum_api_key: str = ""

    # Test Configuration
    test_timeout: int = 30
    max_retries: int = 3
    retry_delay: float = 1.0

    # Data Generation
    num_customers: int = 1000
    num_products: int = 200
    num_orders: int = 5000
    num_biometric_records: int = 1000

    # Performance Testing
    benchmark_iterations: int = 100
    benchmark_warmup: int = 10

    # Logging
    log_level: str = "INFO"

    @property
    def api_base_url(self) -> str:
        """Full API base URL."""
        return f"{self.neuroquantum_api_url}/api/v1"


@lru_cache
def get_settings() -> Settings:
    """Get cached settings instance."""
    return Settings()
