"""HTTP Client for NeuroQuantumDB REST API with retry logic."""

import httpx
from typing import Any, Optional
from tenacity import (
    retry,
    stop_after_attempt,
    wait_exponential,
    retry_if_exception_type,
)
from rich.console import Console

from .config import get_settings
from .models import QueryRequest, QueryResponse, HealthResponse

console = Console()


class NeuroQuantumClient:
    """Async HTTP client for NeuroQuantumDB REST API."""

    def __init__(
        self,
        base_url: Optional[str] = None,
        api_key: Optional[str] = None,
        timeout: float = 30.0,
    ):
        """Initialize the client.

        Args:
            base_url: API base URL (default from settings)
            api_key: API key for authentication (default from settings)
            timeout: Request timeout in seconds
        """
        settings = get_settings()
        self.base_url = base_url or settings.api_base_url
        self.api_key = api_key or settings.neuroquantum_api_key
        self.timeout = timeout

        headers = {}
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"

        self.client = httpx.AsyncClient(
            base_url=self.base_url,
            headers=headers,
            timeout=self.timeout,
        )

    async def __aenter__(self) -> "NeuroQuantumClient":
        """Context manager entry."""
        return self

    async def __aexit__(self, *args: Any) -> None:
        """Context manager exit."""
        await self.close()

    async def close(self) -> None:
        """Close the HTTP client."""
        await self.client.aclose()

    @retry(
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
        retry=retry_if_exception_type(httpx.NetworkError),
    )
    async def health_check(self) -> HealthResponse:
        """Check API health status.

        Returns:
            HealthResponse with status information
        """
        response = await self.client.get("/health")
        response.raise_for_status()
        return HealthResponse(**response.json())

    @retry(
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
        retry=retry_if_exception_type(httpx.NetworkError),
    )
    async def execute_query(self, query: str) -> QueryResponse:
        """Execute a QSQL query.

        Args:
            query: QSQL query string

        Returns:
            QueryResponse with results
        """
        request = QueryRequest(query=query)
        response = await self.client.post(
            "/api/v1/query",
            json=request.model_dump(),
        )

        if response.status_code >= 400:
            error_data = response.json() if response.content else {}
            return QueryResponse(
                success=False,
                error=error_data.get("error", f"HTTP {response.status_code}"),
            )

        data = response.json()
        # Handle the API response structure (data.data contains the actual result)
        if "data" in data:
            actual_data = data["data"]
            return QueryResponse(**actual_data)
        return QueryResponse(**data)

    async def execute_query_raw(self, query: str) -> dict[str, Any]:
        """Execute query and return raw response.

        Args:
            query: QSQL query string

        Returns:
            Raw response dictionary
        """
        request = QueryRequest(query=query)
        response = await self.client.post(
            "/api/v1/query",
            json=request.model_dump(),
        )
        response.raise_for_status()
        return response.json()

    async def batch_execute(self, queries: list[str]) -> list[QueryResponse]:
        """Execute multiple queries sequentially.

        Args:
            queries: List of QSQL query strings

        Returns:
            List of QueryResponse objects
        """
        results = []
        for query in queries:
            result = await self.execute_query(query)
            results.append(result)
        return results
