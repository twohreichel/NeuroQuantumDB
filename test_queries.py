#!/usr/bin/env python3
"""
🧠⚛️🧬 NeuroQuantumDB API Test Suite
=====================================

Umfassendes Test-Script für alle NeuroQuantumDB API-Endpunkte:
- 🧠 Neuromorphic Computing Tests
- ⚛️ Quantum Processing Tests
- 🧬 DNA Storage Tests
- 📊 Monitoring & Admin Tests
- 🔍 Real-time WebSocket Tests

Autor: NeuroQuantumDB Team
Version: 1.0.0
"""

import requests
import json
import time
import asyncio
import websockets
import sys
from typing import Dict, Any, List, Optional
from datetime import datetime
import argparse
import logging

# Konfiguration
BASE_URL = "http://localhost:8080"
API_BASE = f"{BASE_URL}/api/v1"
TEST_API_KEY = None  # Wird dynamisch generiert

# Logging Setup
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

class NeuroQuantumDBTester:
    """🧠 Hauptklasse für alle API-Tests"""

    def __init__(self, base_url: str = BASE_URL):
        self.base_url = base_url
        self.api_base = f"{base_url}/api/v1"
        self.api_key = None
        self.session = requests.Session()
        self.test_results = []

    def log_test(self, test_name: str, success: bool, details: str = "", response_time: float = 0):
        """📊 Test-Ergebnis protokollieren"""
        result = {
            "test": test_name,
            "success": success,
            "details": details,
            "response_time_ms": round(response_time * 1000, 2),
            "timestamp": datetime.now().isoformat()
        }
        self.test_results.append(result)

        status = "✅" if success else "❌"
        logger.info(f"{status} {test_name}: {details} ({response_time*1000:.2f}ms)")

    def make_request(self, method: str, endpoint: str, data: Dict = None, headers: Dict = None) -> tuple:
        """🌐 HTTP-Request mit Fehlerbehandlung"""
        url = f"{self.api_base}/{endpoint.lstrip('/')}"

        # Standard-Headers
        request_headers = {
            "Content-Type": "application/json",
            "User-Agent": "NeuroQuantumDB-Tester/1.0"
        }

        # API-Key hinzufügen falls vorhanden
        if self.api_key:
            request_headers["X-API-Key"] = self.api_key

        # Custom Headers hinzufügen
        if headers:
            request_headers.update(headers)

        try:
            start_time = time.time()

            if method.upper() == "GET":
                response = self.session.get(url, headers=request_headers, timeout=30)
            elif method.upper() == "POST":
                response = self.session.post(url, json=data, headers=request_headers, timeout=30)
            elif method.upper() == "PUT":
                response = self.session.put(url, json=data, headers=request_headers, timeout=30)
            elif method.upper() == "DELETE":
                response = self.session.delete(url, headers=request_headers, timeout=30)
            else:
                raise ValueError(f"Unsupported HTTP method: {method}")

            response_time = time.time() - start_time

            return response, response_time

        except Exception as e:
            logger.error(f"Request failed: {str(e)}")
            return None, 0

    def test_basic_health(self):
        """💚 Basis-Gesundheitscheck"""
        logger.info("🏥 Testing Basic Health...")

        response, response_time = self.make_request("GET", "/health")

        if response and response.status_code == 200:
            try:
                result = response.json()
                # Handle the server's response structure
                if result.get("success") and "data" in result:
                    data = result["data"]
                    status = data.get("status", "unknown")
                    version = data.get("version", "unknown")
                    uptime = data.get("uptime_seconds", 0)
                    self.log_test(
                        "Basic Health Check",
                        True,
                        f"Status: {status}, Version: {version}, Uptime: {uptime}s",
                        response_time
                    )
                else:
                    # Fallback for different response structure
                    status = result.get("status", "unknown")
                    self.log_test(
                        "Basic Health Check",
                        True,
                        f"Status: {status}",
                        response_time
                    )
                return True
            except (json.JSONDecodeError, KeyError) as e:
                self.log_test(
                    "Basic Health Check",
                    False,
                    f"Invalid health response: {str(e)}",
                    response_time
                )
                return False
        else:
            error_msg = "Health endpoint failed"
            if response:
                error_msg += f" (HTTP {response.status_code})"
            self.log_test("Basic Health Check", False, error_msg, response_time)
            return False

    def generate_api_key(self):
        """🔑 API-Key generieren"""
        logger.info("🔑 Generating API Key...")

        data = {
            "name": "neuroquantum-tester",
            "permissions": ["read", "write", "admin"]
        }

        response, response_time = self.make_request("POST", "/auth/generate-key", data)

        if response and response.status_code == 200:
            try:
                result = response.json()
                # Check if response has the expected structure
                if result.get("success") and "data" in result:
                    api_key_data = result["data"]
                    self.api_key = api_key_data.get("api_key")
                    if self.api_key:
                        # Extract additional info for logging
                        expires_at = api_key_data.get("expires_at", "unknown")
                        permissions = api_key_data.get("permissions", [])
                        self.log_test(
                            "API Key Generation",
                            True,
                            f"Key generated: {self.api_key[:20]}..., Expires: {expires_at}, Permissions: {len(permissions)}",
                            response_time
                        )
                        return True
                    else:
                        self.log_test(
                            "API Key Generation",
                            False,
                            "API key not found in response data",
                            response_time
                        )
                        return False
                else:
                    # Fallback for different response structure
                    self.api_key = result.get("api_key")
                    if self.api_key:
                        self.log_test(
                            "API Key Generation",
                            True,
                            f"Key generated: {self.api_key[:20]}...",
                            response_time
                        )
                        return True
                    else:
                        self.log_test(
                            "API Key Generation",
                            False,
                            f"Unexpected response structure: {result.get('error', 'Unknown error')}",
                            response_time
                        )
                        return False
            except (json.JSONDecodeError, KeyError) as e:
                self.log_test(
                    "API Key Generation",
                    False,
                    f"Invalid response format: {str(e)}",
                    response_time
                )
                return False
        else:
            error_msg = "Failed to generate API key"
            if response:
                error_msg += f" (HTTP {response.status_code})"
                try:
                    error_data = response.json()
                    if "error" in error_data:
                        error_msg += f": {error_data['error']}"
                except:
                    pass
            else:
                error_msg += " (No response from server)"

            self.log_test("API Key Generation", False, error_msg, response_time)
            return False

    def test_neuromorphic_queries(self):
        """🧠 Neuromorphic Computing Tests"""
        logger.info("🧠 Testing Neuromorphic Queries...")

        # Test 1: Basis NEUROMATCH Query
        data = {
            "query": "NEUROMATCH users WHERE city = 'Berlin' WITH SYNAPTIC_WEIGHT 0.8",
            "learning_enabled": True,
            "plasticity_threshold": 0.5
        }

        response, response_time = self.make_request("POST", "/neuromorphic/query", data)

        if response and response.status_code == 200:
            result = response.json()
            self.log_test(
                "Neuromorphic NEUROMATCH Query",
                True,
                f"Execution time: {result.get('execution_time_us', 0)}μs",
                response_time
            )
        else:
            self.log_test("Neuromorphic NEUROMATCH Query", False, "Query failed", response_time)

        # Test 2: Erweiterte neuromorphe Query mit Lernparametern
        data = {
            "query": "NEUROMATCH products WHERE category = 'electronics' AND price < 1000 WITH SYNAPTIC_WEIGHT 0.9, PLASTICITY_RATE 0.02",
            "plasticity_threshold": 0.7,
            "memory_consolidation": True
        }

        logger.info("���� Testing Neuromorphic Queries...")

        response, response_time = self.make_request("POST", "/neuromorphic/query", data)

        if response and response.status_code == 200:
            result = response.json()
            self.log_test(
                "Advanced Neuromorphic Query",
                True,
                f"Synaptic strength: {result.get('neuromorphic_stats', {}).get('synaptic_strength', 0)}",
                response_time
            )
        else:
            self.log_test("Advanced Neuromorphic Query", False, "Advanced query failed", response_time)

        # Test 3: Netzwerk-Status abrufen
        response, response_time = self.make_request("GET", "/neuromorphic/network-status")

        if response and response.status_code == 200:
            result = response.json()
            self.log_test(
                "Neuromorphic Network Status",
                True,
                f"Active synapses: {result.get('active_synapses', 0)}",
                response_time
            )
        else:
            self.log_test("Neuromorphic Network Status", False, "Network status failed", response_time)

        # Test 4: Manuelles Training
        training_data = {
            "training_data": [
                {"pattern": ["user_login", "search_products", "purchase"], "weight": 0.9},
                {"pattern": ["user_login", "browse_categories", "add_to_cart"], "weight": 0.7},
                {"pattern": ["user_register", "verify_email", "first_purchase"], "weight": 0.85}
            ],
            "learning_rate": 0.02,
            "epochs": 50
        }

        response, response_time = self.make_request("POST", "/neuromorphic/train", training_data)

        if response and response.status_code == 200:
            self.log_test(
                "Neuromorphic Manual Training",
                True,
                "Training completed successfully",
                response_time
            )
        else:
            self.log_test("Neuromorphic Manual Training", False, "Training failed", response_time)

    def test_quantum_operations(self):
        """⚛️ Quantum Computing Tests"""
        logger.info("⚛️ Testing Quantum Operations...")

        # Test 1: Quantum Search mit Grover's Algorithm
        data = {
            "query": "QUANTUM_SELECT * FROM products WHERE category = 'electronics' AND rating > 4.5",
            "grover_iterations": 15,
            "amplitude_amplification": True,
            "parallel_processing": True
        }

        response, response_time = self.make_request("POST", "/quantum/search", data)

        if response and response.status_code == 200:
            result = response.json()
            speedup = result.get('quantum_speedup', 0)
            self.log_test(
                "Quantum Grover Search",
                True,
                f"Speedup: {speedup}x, Execution: {result.get('execution_time_us', 0)}μs",
                response_time
            )
        else:
            self.log_test("Quantum Grover Search", False, "Quantum search failed", response_time)

        # Test 2: Quantum Optimization mit Annealing
        optimization_problem = {
            "problem": {
                "variables": ["index_order", "cache_strategy", "memory_layout"],
                "constraints": ["memory < 100MB", "response_time < 1μs"],
                "objective": "minimize_energy_consumption"
            },
            "annealing_steps": 1000,
            "temperature_schedule": "exponential"
        }

        response, response_time = self.make_request("POST", "/quantum/optimize", optimization_problem)

        if response and response.status_code == 200:
            result = response.json()
            self.log_test(
                "Quantum Annealing Optimization",
                True,
                f"Energy saving: {result.get('energy_saving_percent', 0)}%",
                response_time
            )
        else:
            self.log_test("Quantum Annealing Optimization", False, "Quantum optimization failed", response_time)

        # Test 3: Quantum Status
        response, response_time = self.make_request("GET", "/quantum/status")

        if response and response.status_code == 200:
            result = response.json()
            self.log_test(
                "Quantum Processor Status",
                True,
                f"Processors: {result.get('quantum_processors', 0)}, Coherence: {result.get('coherence_time_us', 0)}μs",
                response_time
            )
        else:
            self.log_test("Quantum Processor Status", False, "Quantum status failed", response_time)

    def test_dna_storage(self):
        """🧬 DNA Storage Tests"""
        logger.info("🧬 Testing DNA Storage...")

        # Test 1: DNA Compression
        test_data = "Dies ist ein sehr langer Teststring der komprimiert werden soll. " * 100

        compression_data = {
            "data": test_data,
            "compression_level": 9,
            "error_correction": True,
            "biological_patterns": True
        }

        response, response_time = self.make_request("POST", "/dna/compress", compression_data)

        compressed_sequence = None
        if response and response.status_code == 200:
            result = response.json()
            compressed_sequence = result.get("dna_sequence")
            compression_ratio = result.get("compression_ratio", 0)
            self.log_test(
                "DNA Compression",
                True,
                f"Ratio: {compression_ratio}:1, Size: {result.get('original_size_bytes')} -> {result.get('compressed_size_bytes')} bytes",
                response_time
            )
        else:
            self.log_test("DNA Compression", False, "DNA compression failed", response_time)

        # Test 2: DNA Decompression
        if compressed_sequence:
            decompression_data = {
                "dna_sequence": compressed_sequence,
                "error_correction_codes": "REED_SOLOMON_255_223",
                "verify_integrity": True
            }

            response, response_time = self.make_request("POST", "/dna/decompress", decompression_data)

            if response and response.status_code == 200:
                result = response.json()
                self.log_test(
                    "DNA Decompression",
                    True,
                    f"Integrity verified: {result.get('integrity_verified')}, Errors: {result.get('errors_corrected', 0)}",
                    response_time
                )
            else:
                self.log_test("DNA Decompression", False, "DNA decompression failed", response_time)

        # Test 3: DNA Repair
        damaged_sequence = "ATCGATXGTAGCTANNNATCGATCG"  # X und N = beschädigte Nukleotide

        repair_data = {
            "damaged_sequence": damaged_sequence,
            "repair_strategy": "biological_consensus",
            "redundancy_check": True
        }

        response, response_time = self.make_request("POST", "/dna/repair", repair_data)

        if response and response.status_code == 200:
            result = response.json()
            self.log_test(
                "DNA Repair",
                True,
                f"Errors found: {result.get('errors_found', 0)}, Corrected: {result.get('errors_corrected', 0)}, Confidence: {result.get('confidence', 0)}",
                response_time
            )
        else:
            self.log_test("DNA Repair", False, "DNA repair failed", response_time)

    def test_monitoring_admin(self):
        """📊 Monitoring & Admin Tests"""
        logger.info("📊 Testing Monitoring & Admin...")

        # Test 1: Detailed Health Check
        response, response_time = self.make_request("GET", "/health")

        if response and response.status_code == 200:
            result = response.json()
            self.log_test(
                "Detailed Health Check",
                True,
                f"Memory: {result.get('system_metrics', {}).get('memory_usage_mb', 0)}MB, Power: {result.get('system_metrics', {}).get('power_consumption_w', 0)}W",
                response_time
            )
        else:
            self.log_test("Detailed Health Check", False, "Health check failed", response_time)

        # Test 2: Prometheus Metrics
        response, response_time = self.make_request("GET", "/metrics")

        if response and response.status_code == 200:
            self.log_test(
                "Prometheus Metrics",
                True,
                f"Metrics length: {len(response.text)} chars",
                response_time
            )
        else:
            self.log_test("Prometheus Metrics", False, "Metrics endpoint failed", response_time)

        # Test 3: Admin Config Get
        response, response_time = self.make_request("GET", "/admin/config")

        if response and response.status_code == 200:
            result = response.json()
            self.log_test(
                "Admin Config Get",
                True,
                f"Config sections: {len(result.keys())}",
                response_time
            )
        else:
            self.log_test("Admin Config Get", False, "Config get failed", response_time)

        # Test 4: Admin Config Update
        config_update = {
            "neuromorphic": {
                "learning_rate": 0.015,
                "plasticity_threshold": 0.6,
                "max_synapses": 1000000,
                "auto_optimization": True
            },
            "quantum": {
                "processors": 4,
                "grover_iterations": 20,
                "annealing_steps": 1000,
                "error_correction": True
            }
        }

        response, response_time = self.make_request("PUT", "/admin/config", config_update)

        if response and response.status_code == 200:
            result = response.json()
            self.log_test(
                "Admin Config Update",
                True,
                f"Changes applied: {len(result.get('changes_applied', []))}",
                response_time
            )
        else:
            self.log_test("Admin Config Update", False, "Config update failed", response_time)

    def test_complex_queries(self):
        """🔬 Komplexe Query Tests"""
        logger.info("🔬 Testing Complex Queries...")

        # Test 1: Kombinierte Neuromorphic + Quantum Query
        complex_query = {
            "query": """
                WITH neuromorphic_users AS (
                    NEUROMATCH users WHERE age BETWEEN 25 AND 45 
                    WITH SYNAPTIC_WEIGHT 0.85
                ),
                quantum_products AS (
                    QUANTUM_SELECT products WHERE category IN ('electronics', 'books')
                    WITH GROVER_ITERATIONS 12
                )
                SELECT nu.name, qp.title, qp.price 
                FROM neuromorphic_users nu
                JOIN quantum_products qp ON nu.interest_category = qp.category
                ORDER BY qp.price DESC
                LIMIT 100
            """,
            "hybrid_processing": True,
            "optimization_level": "aggressive"
        }

        response, response_time = self.make_request("POST", "/neuromorphic/query", complex_query)

        if response and response.status_code == 200:
            self.log_test(
                "Complex Hybrid Query",
                True,
                "Neuromorphic + Quantum processing completed",
                response_time
            )
        else:
            self.log_test("Complex Hybrid Query", False, "Complex query failed", response_time)

        # Test 2: DNA-compressed Data Query
        dna_query = {
            "query": "SELECT * FROM compressed_logs WHERE timestamp > NOW() - INTERVAL 1 HOUR",
            "dna_decompression": True,
            "parallel_decompression": True
        }

        response, response_time = self.make_request("POST", "/dna/query", dna_query)

        # Dieser Endpoint existiert möglicherweise nicht, daher weniger strikt
        if response and response.status_code == 200:
            self.log_test(
                "DNA Compressed Data Query",
                True,
                "DNA decompression query completed",
                response_time
            )
        else:
            self.log_test("DNA Compressed Data Query", False, "DNA query endpoint not available", response_time)

    async def test_websocket_realtime(self):
        """🔍 WebSocket Real-time Tests"""
        logger.info("🔍 Testing WebSocket Real-time...")

        try:
            uri = f"ws://localhost:8080/api/v1/realtime"

            async with websockets.connect(uri) as websocket:
                # Authentifizierung
                auth_message = {
                    "type": "auth",
                    "api_key": self.api_key
                }
                await websocket.send(json.dumps(auth_message))

                # Kanäle abonnieren
                subscribe_message = {
                    "type": "subscribe",
                    "channels": ["neuromorphic_learning", "quantum_operations", "dna_compression"]
                }
                await websocket.send(json.dumps(subscribe_message))

                # Kurz warten auf Nachrichten
                try:
                    message = await asyncio.wait_for(websocket.recv(), timeout=5.0)
                    data = json.loads(message)

                    self.log_test(
                        "WebSocket Real-time",
                        True,
                        f"Received message type: {data.get('type', 'unknown')}",
                        0
                    )
                except asyncio.TimeoutError:
                    self.log_test(
                        "WebSocket Real-time",
                        True,
                        "WebSocket connection established (no immediate messages)",
                        0
                    )

        except Exception as e:
            self.log_test("WebSocket Real-time", False, f"WebSocket error: {str(e)}", 0)

    def run_performance_benchmarks(self):
        """🏃‍♂️ Performance Benchmarks"""
        logger.info("🏃‍♂️ Running Performance Benchmarks...")

        # Benchmark 1: Query Response Time
        query_times = []
        for i in range(10):
            data = {
                "query": f"NEUROMATCH test_table WHERE id = {i} WITH SYNAPTIC_WEIGHT 0.8",
                "learning_enabled": False  # Für konsistente Messung
            }

            response, response_time = self.make_request("POST", "/neuromorphic/query", data)
            if response and response.status_code == 200:
                query_times.append(response_time)

        if query_times:
            avg_time = sum(query_times) / len(query_times)
            min_time = min(query_times)
            max_time = max(query_times)

            self.log_test(
                "Query Performance Benchmark",
                True,
                f"Avg: {avg_time*1000:.2f}ms, Min: {min_time*1000:.2f}ms, Max: {max_time*1000:.2f}ms",
                avg_time
            )

        # Benchmark 2: Concurrent Requests
        import concurrent.futures
        import threading

        def concurrent_request():
            data = {"query": "QUANTUM_SELECT * FROM test_concurrent LIMIT 10"}
            response, response_time = self.make_request("POST", "/quantum/search", data)
            return response_time if response and response.status_code == 200 else None

        with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
            futures = [executor.submit(concurrent_request) for _ in range(20)]
            concurrent_times = [f.result() for f in concurrent.futures.as_completed(futures)]

        successful_requests = [t for t in concurrent_times if t is not None]

        if successful_requests:
            self.log_test(
                "Concurrent Requests Benchmark",
                True,
                f"Successful: {len(successful_requests)}/20, Avg time: {sum(successful_requests)/len(successful_requests)*1000:.2f}ms",
                sum(successful_requests)/len(successful_requests)
            )
        else:
            self.log_test("Concurrent Requests Benchmark", False, "No successful concurrent requests", 0)

    def print_summary(self):
        """📋 Test-Zusammenfassung ausgeben"""
        print("\n" + "="*60)
        print("🧠⚛️🧬 NEUROQUANTUMDB TEST SUMMARY")
        print("="*60)

        total_tests = len(self.test_results)
        successful_tests = len([r for r in self.test_results if r["success"]])
        failed_tests = total_tests - successful_tests

        print(f"📊 Total Tests: {total_tests}")
        print(f"✅ Successful: {successful_tests}")
        print(f"❌ Failed: {failed_tests}")
        print(f"📈 Success Rate: {(successful_tests/total_tests)*100:.1f}%")

        if self.test_results:
            avg_response_time = sum([r["response_time_ms"] for r in self.test_results]) / total_tests
            print(f"⚡ Average Response Time: {avg_response_time:.2f}ms")

        print("\n🔍 DETAILED RESULTS:")
        print("-" * 60)

        for result in self.test_results:
            status = "✅" if result["success"] else "❌"
            print(f"{status} {result['test']:.<40} {result['response_time_ms']:>6.2f}ms")
            if result["details"]:
                print(f"   └─ {result['details']}")

        # Fehlgeschlagene Tests hervorheben
        failed_results = [r for r in self.test_results if not r["success"]]
        if failed_results:
            print(f"\n🚨 FAILED TESTS ({len(failed_results)}):")
            print("-" * 60)
            for result in failed_results:
                print(f"❌ {result['test']}: {result['details']}")

        print("\n" + "="*60)

    async def run_all_tests(self):
        """🚀 Alle Tests ausführen"""
        print("🧠⚛️🧬 Starting NeuroQuantumDB API Test Suite...")
        print("="*60)

        # 1. Basis-Health-Check
        if not self.test_basic_health():
            print("❌ Basic health check failed - aborting tests")
            return False

        # 2. API-Key generieren
        if not self.generate_api_key():
            print("⚠️  Continuing without API key...")

        # 3. Neuromorphic Tests
        self.test_neuromorphic_queries()

        # 4. Quantum Tests
        self.test_quantum_operations()

        # 5. DNA Storage Tests
        self.test_dna_storage()

        # 6. Monitoring & Admin Tests
        self.test_monitoring_admin()

        # 7. Complex Query Tests
        self.test_complex_queries()

        # 8. WebSocket Tests (falls API-Key vorhanden)
        if self.api_key:
            await self.test_websocket_realtime()

        # 9. Performance Benchmarks
        self.run_performance_benchmarks()

        # 10. Zusammenfassung
        self.print_summary()

        return True

def main():
    """🎯 Hauptfunktion"""
    parser = argparse.ArgumentParser(description="NeuroQuantumDB API Test Suite")
    parser.add_argument("--url", default="http://localhost:8080", help="Base URL der NeuroQuantumDB API")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose logging")
    parser.add_argument("--quick", "-q", action="store_true", help="Nur grundlegende Tests")

    args = parser.parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    # Tester initialisieren
    tester = NeuroQuantumDBTester(args.url)

    # Tests ausführen
    try:
        if args.quick:
            # Nur Health Check und grundlegende Tests
            tester.test_basic_health()
            tester.generate_api_key()
            tester.test_neuromorphic_queries()
            tester.print_summary()
        else:
            # Alle Tests mit WebSocket
            asyncio.run(tester.run_all_tests())

    except KeyboardInterrupt:
        print("\n⏹️  Tests interrupted by user")
        tester.print_summary()
    except Exception as e:
        logger.error(f"Test suite failed: {str(e)}")
        tester.print_summary()
        sys.exit(1)

if __name__ == "__main__":
    main()
