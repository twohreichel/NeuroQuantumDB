#!/usr/bin/env python3
"""
NeuroQuantumDB REST API Demo Client
Demonstrates quantum-enhanced database operations via REST API
"""

import asyncio
import aiohttp
import json
import time
from typing import Dict, Any, Optional

class NeuroQuantumDBClient:
    """Client for interacting with NeuroQuantumDB REST API"""

    def __init__(self, base_url: str = "http://localhost:8080"):
        self.base_url = base_url
        self.session: Optional[aiohttp.ClientSession] = None
        self.auth_token: Optional[str] = None

    async def __aenter__(self):
        self.session = aiohttp.ClientSession()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self.session:
            await self.session.close()

    async def health_check(self) -> Dict[str, Any]:
        """Check API health and system metrics"""
        async with self.session.get(f"{self.base_url}/api/v1/health") as resp:
            return await resp.json()

    async def login(self, username: str = "demo", password: str = "quantum123") -> Dict[str, Any]:
        """Authenticate with quantum-resistant security"""
        login_data = {
            "username": username,
            "password": password,
            "quantum_challenge": "demo_challenge"
        }

        async with self.session.post(
            f"{self.base_url}/api/v1/auth/login",
            json=login_data
        ) as resp:
            result = await resp.json()
            if "access_token" in result:
                self.auth_token = result["access_token"]
            return result

    async def quantum_search(
        self,
        query: str,
        quantum_level: int = 128,
        use_grovers: bool = True,
        limit: int = 10
    ) -> Dict[str, Any]:
        """Execute quantum-enhanced search"""
        headers = {}
        if self.auth_token:
            headers["Authorization"] = f"Bearer {self.auth_token}"

        params = {
            "query": query,
            "quantum_level": quantum_level,
            "use_grovers": use_grovers,
            "limit": limit
        }

        async with self.session.get(
            f"{self.base_url}/api/v1/quantum-search",
            params=params,
            headers=headers
        ) as resp:
            return await resp.json()

    async def execute_qsql(
        self,
        query: str,
        optimize: bool = True,
        explain: bool = False
    ) -> Dict[str, Any]:
        """Execute QSQL query with neuromorphic optimization"""
        headers = {"Content-Type": "application/json"}
        if self.auth_token:
            headers["Authorization"] = f"Bearer {self.auth_token}"

        qsql_data = {
            "query": query,
            "optimize": optimize,
            "explain": explain
        }

        async with self.session.post(
            f"{self.base_url}/api/v1/qsql/execute",
            json=qsql_data,
            headers=headers
        ) as resp:
            return await resp.json()

    async def get_schema(self) -> Dict[str, Any]:
        """Get database schema with synaptic network information"""
        headers = {}
        if self.auth_token:
            headers["Authorization"] = f"Bearer {self.auth_token}"

        async with self.session.get(
            f"{self.base_url}/api/v1/schema",
            headers=headers
        ) as resp:
            return await resp.json()

    async def get_metrics(self) -> str:
        """Get Prometheus metrics"""
        async with self.session.get(f"{self.base_url}/api/v1/metrics") as resp:
            return await resp.text()

async def demo_api_interactions():
    """Demonstrate various API features"""
    print("🧠 NeuroQuantumDB REST API Demo")
    print("=" * 50)

    async with NeuroQuantumDBClient() as client:
        try:
            # 1. Health Check
            print("\n1. 🏥 Health Check")
            health = await client.health_check()
            print(f"   Status: {health.get('status', 'unknown')}")
            print(f"   Version: {health.get('version', 'unknown')}")
            print(f"   Power Consumption: {health.get('power_consumption_mw', 0)}mW")

            # 2. Authentication
            print("\n2. 🔐 Quantum Authentication")
            auth_result = await client.login()
            if auth_result.get("success"):
                print(f"   ✅ Authentication successful")
                print(f"   Quantum Level: {auth_result.get('quantum_level', 0)}")
            else:
                print(f"   ❌ Authentication failed")
                return

            # 3. Quantum Search
            print("\n3. ⚛️ Quantum-Enhanced Search")
            search_result = await client.quantum_search(
                query="neural networks",
                quantum_level=192,
                use_grovers=True
            )
            if search_result.get("success"):
                data = search_result.get("data", {})
                print(f"   Results found: {len(data.get('results', []))}")
                print(f"   Quantum speedup: {data.get('quantum_speedup', 0)}x")
                print(f"   Compression savings: {data.get('compression_savings', 0)}:1")

            # 4. QSQL Execution
            print("\n4. 🧬 QSQL Query Execution")
            qsql_query = """
                NEUROMATCH products 
                WHERE price < 100 
                WITH SYNAPTIC_WEIGHT 0.8
                QUANTUM_JOIN categories ON products.category_id = categories.id
            """
            qsql_result = await client.execute_qsql(qsql_query, optimize=True, explain=True)
            if qsql_result.get("success"):
                data = qsql_result.get("data", {})
                metrics = data.get("performance_metrics", {})
                print(f"   Execution time: {metrics.get('execution_time_us', 0)}μs")
                print(f"   Memory usage: {metrics.get('memory_usage_mb', 0)}MB")
                print(f"   Power consumption: {metrics.get('power_consumption_mw', 0)}mW")
                print(f"   Quantum operations: {metrics.get('quantum_operations', 0)}")

            # 5. Schema Information
            print("\n5. 📊 Database Schema")
            schema = await client.get_schema()
            print(f"   Tables: {len(schema.get('tables', []))}")
            print(f"   Synaptic Networks: {len(schema.get('synaptic_networks', []))}")
            print(f"   Quantum Indexes: {len(schema.get('quantum_indexes', []))}")

            # 6. Performance Metrics
            print("\n6. 📈 System Metrics")
            metrics_text = await client.get_metrics()
            lines = metrics_text.split('\n')
            for line in lines:
                if 'neuroquantum_' in line and not line.startswith('#'):
                    print(f"   {line}")

        except aiohttp.ClientError as e:
            print(f"❌ Connection error: {e}")
            print("   Make sure NeuroQuantumDB API server is running on localhost:8080")
        except Exception as e:
            print(f"❌ Error: {e}")

if __name__ == "__main__":
    print("Starting NeuroQuantumDB API Demo...")
    asyncio.run(demo_api_interactions())
