"""Configuration management for NeuroQuantumDB integration tests."""

from pydantic_settings import BaseSettings, SettingsConfigDict
from functools import lru_cache


class Settings(BaseSettings):
    """Application settings loaded from environment variables."""

    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
        extra="ignore",
    )

    # API Configuration
    neuroquantum_api_url: str = "http://localhost:8080"
    neuroquantum_api_key: str = ""

    # Test Configuration
    test_timeout: int = 30
    max_retries: int = 3
    retry_delay: float = 1.0

    # Data Generation Settings
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
        """Get the base URL for API requests."""
        return self.neuroquantum_api_url.rstrip("/")


@lru_cache()
def get_settings() -> Settings:
    """Get cached settings instance.

    Returns:
        Settings: The application settings
    """
    return Settings()
