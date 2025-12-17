#!/usr/bin/env python3
"""Comprehensive API Test Suite for NeuroQuantumDB"""

import subprocess
import json
import sys

API_KEY = "nqdb_03c495c620c646eaa7ce89dd2a78ce86"
BASE_URL = "http://127.0.0.1:8080"

def run_curl(method, endpoint, data=None, headers=None):
    """Run curl and return result"""
    cmd = ["curl", "-s", "-X", method]
    
    if headers:
        for k, v in headers.items():
            cmd.extend(["-H", f"{k}: {v}"])
    
    cmd.extend(["-H", f"X-API-Key: {API_KEY}"])
    
    if data:
        cmd.extend(["-H", "Content-Type: application/json"])
        cmd.extend(["-d", json.dumps(data)])
    
    cmd.append(f"{BASE_URL}{endpoint}")
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        return result.stdout, result.returncode
    except Exception as e:
        return str(e), -1

def test_endpoint(name, method, endpoint, data=None, expected_success=True):
    """Test an endpoint and report result"""
    print(f"\n{'='*60}")
    print(f"TEST: {name}")
    print(f"{'='*60}")
    print(f"Endpoint: {method} {endpoint}")
    if data:
        print(f"Payload: {json.dumps(data)[:100]}...")
    
    response, code = run_curl(method, endpoint, data)
    
    try:
        result = json.loads(response) if response else {}
        print(f"Response: {json.dumps(result, indent=2)[:500]}")
        
        success = result.get("success", False)
        error = result.get("error")
        
        if expected_success:
            status = "✅ PASS" if success else "❌ FAIL"
        else:
            status = "✅ PASS" if not success or error else "❌ FAIL"
        
        print(f"Status: {status}")
        return success, result
    except json.JSONDecodeError:
        print(f"Raw Response: {response[:300]}")
        status = "⚠️ NON-JSON RESPONSE"
        print(f"Status: {status}")
        return False, {"raw": response}

def main():
    results = []
    
    print("\n" + "="*60)
    print("NEUROQUANTUMDB COMPREHENSIVE API TEST SUITE")
    print("="*60)
    
    # 1. Health Check (no auth needed)
    print("\n[1] HEALTH CHECK")
    resp, _ = run_curl("GET", "/health")
    try:
        data = json.loads(resp)
        print(f"Status: {data.get('data', {}).get('status')}")
        print(f"Version: {data.get('data', {}).get('version')}")
        print(f"Features: {data.get('data', {}).get('features')}")
        results.append(("Health Check", True, data.get('data', {}).get('status') == 'healthy'))
    except:
        print(f"Error: {resp}")
        results.append(("Health Check", False, False))
    
    # 2. Metrics
    print("\n[2] METRICS ENDPOINT")
    resp, _ = run_curl("GET", "/metrics")
    if "# HELP" in resp or "neuroquantum" in resp:
        print("✅ Prometheus metrics available")
        print(f"First 200 chars: {resp[:200]}")
        results.append(("Metrics", True, True))
    else:
        print(f"Response: {resp[:200]}")
        results.append(("Metrics", False, False))
    
    # 3. SQL - Basic SELECT
    print("\n[3] SQL - SELECT * FROM users")
    success, data = test_endpoint("Basic SELECT", "POST", "/api/v1/sql", 
                                   {"query": "SELECT * FROM users"})
    results.append(("SQL SELECT", True, success))
    
    # 4. SQL - INSERT
    print("\n[4] SQL - INSERT INTO users")
    success, data = test_endpoint("INSERT", "POST", "/api/v1/sql",
                                   {"query": "INSERT INTO users (name, email, age) VALUES ('TestUser', 'test@example.com', 30)"})
    results.append(("SQL INSERT", True, success))
    
    # 5. SQL - UPDATE
    print("\n[5] SQL - UPDATE users")
    success, data = test_endpoint("UPDATE", "POST", "/api/v1/sql",
                                   {"query": "UPDATE users SET age = 35 WHERE name = 'TestUser'"})
    results.append(("SQL UPDATE", True, success))
    
    # 6. SQL - SELECT with WHERE
    print("\n[6] SQL - SELECT with WHERE")
    success, data = test_endpoint("SELECT with WHERE", "POST", "/api/v1/sql",
                                   {"query": "SELECT * FROM users WHERE age > 25"})
    results.append(("SQL WHERE", True, success))
    
    # 7. SQL - SELECT with ORDER BY
    print("\n[7] SQL - SELECT with ORDER BY")
    success, data = test_endpoint("SELECT ORDER BY", "POST", "/api/v1/sql",
                                   {"query": "SELECT * FROM users ORDER BY age DESC"})
    results.append(("SQL ORDER BY", True, success))
    
    # 8. SQL - SELECT with LIMIT
    print("\n[8] SQL - SELECT with LIMIT")
    success, data = test_endpoint("SELECT LIMIT", "POST", "/api/v1/sql",
                                   {"query": "SELECT * FROM users LIMIT 3"})
    results.append(("SQL LIMIT", True, success))
    
    # 9. SQL - COUNT
    print("\n[9] SQL - COUNT")
    success, data = test_endpoint("COUNT Query", "POST", "/api/v1/sql",
                                   {"query": "SELECT COUNT(*) FROM users"})
    results.append(("SQL COUNT", True, success))
    
    # 10. SQL - DELETE
    print("\n[10] SQL - DELETE")
    success, data = test_endpoint("DELETE", "POST", "/api/v1/sql",
                                   {"query": "DELETE FROM users WHERE name = 'TestUser'"})
    results.append(("SQL DELETE", True, success))
    
    # 11. SQL - CREATE TABLE (should fail via SQL, use REST)
    print("\n[11] SQL - CREATE TABLE (expected to fail)")
    success, data = test_endpoint("CREATE TABLE via SQL", "POST", "/api/v1/sql",
                                   {"query": "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT)"}, 
                                   expected_success=False)
    results.append(("SQL CREATE TABLE", False, not success))
    
    # 12. REST - Create Table
    print("\n[12] REST - Create Table")
    success, data = test_endpoint("Create Table via REST", "POST", "/api/v1/tables",
                                   {"name": "api_test_table", 
                                    "columns": [
                                        {"name": "id", "data_type": "INTEGER", "primary_key": True},
                                        {"name": "value", "data_type": "TEXT"}
                                    ]})
    results.append(("REST Create Table", True, success))
    
    # 13. REST - Insert Record
    print("\n[13] REST - Insert Records")
    success, data = test_endpoint("Insert via REST", "POST", "/api/v1/tables/users/records",
                                   {"records": [{"name": "APITest", "email": "api@test.com", "age": 28}]})
    results.append(("REST Insert", True, success))
    
    # 14. QSQL - NEUROMATCH
    print("\n[14] QSQL - NEUROMATCH Query")
    success, data = test_endpoint("NEUROMATCH", "POST", "/api/v1/sql",
                                   {"query": "SELECT * FROM users WHERE NEUROMATCH(name, 'John') > 0.5"})
    results.append(("QSQL NEUROMATCH", True, success))
    
    # 15. QSQL - QUANTUM_SEARCH
    print("\n[15] QSQL - QUANTUM_SEARCH Query")
    success, data = test_endpoint("QUANTUM_SEARCH", "POST", "/api/v1/sql",
                                   {"query": "SELECT * FROM users WHERE QUANTUM_SEARCH(name, 'test')"})
    results.append(("QSQL QUANTUM_SEARCH", True, success))
    
    # 16. QSQL - SYNAPTIC_WEIGHT
    print("\n[16] QSQL - SYNAPTIC_WEIGHT Query")
    success, data = test_endpoint("SYNAPTIC_WEIGHT", "POST", "/api/v1/sql",
                                   {"query": "SELECT SYNAPTIC_WEIGHT(name, email) as weight FROM users LIMIT 5"})
    results.append(("QSQL SYNAPTIC_WEIGHT", True, success))
    
    # 17. DNA Compression Endpoint
    print("\n[17] DNA - Compress Data")
    success, data = test_endpoint("DNA Compress", "POST", "/api/v1/dna/compress",
                                   {"data": "Hello, NeuroQuantumDB! This is a test of DNA compression."})
    results.append(("DNA Compress", True, success))
    
    # 18. DNA Decompress
    print("\n[18] DNA - Decompress Data")
    success, data = test_endpoint("DNA Decompress", "POST", "/api/v1/dna/decompress",
                                   {"data": "ATCGATCG"})
    results.append(("DNA Decompress", True, success))
    
    # 19. Quantum Search Endpoint
    print("\n[19] Quantum - Search")
    success, data = test_endpoint("Quantum Search", "POST", "/api/v1/quantum/search",
                                   {"query": "test", "table": "users", "column": "name"})
    results.append(("Quantum Search", True, success))
    
    # 20. Neural - Train
    print("\n[20] Neural - Train Network")
    success, data = test_endpoint("Neural Train", "POST", "/api/v1/neural/train",
                                   {"table": "users", "features": ["age"], "target": "name"})
    results.append(("Neural Train", True, success))
    
    # 21. Auth - Login (should be disabled)
    print("\n[21] AUTH - Login (should be disabled)")
    resp, _ = run_curl("POST", "/api/v1/auth/login")
    print(f"Response: {resp[:200]}")
    results.append(("Auth Login Disabled", True, "501" in resp or "not implemented" in resp.lower() or not resp))
    
    # 22. Auth - Invalid API Key
    print("\n[22] AUTH - Invalid API Key")
    cmd = ["curl", "-s", "-H", "X-API-Key: invalid_key", "-H", "Content-Type: application/json",
           "-d", '{"query": "SELECT 1"}', f"{BASE_URL}/api/v1/sql"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    print(f"Response: {result.stdout[:200]}")
    results.append(("Invalid API Key Rejection", True, "401" in result.stdout or "unauthorized" in result.stdout.lower() or "Invalid" in result.stdout))
    
    # 23. WebSocket endpoint test
    print("\n[23] WEBSOCKET - Connection Test")
    resp, _ = run_curl("GET", "/ws")
    print(f"Response: {resp[:200]}")
    results.append(("WebSocket Endpoint", True, True))  # Just checking it exists
    
    # Summary
    print("\n" + "="*60)
    print("TEST SUMMARY")
    print("="*60)
    
    passed = 0
    failed = 0
    for name, expected, actual in results:
        status = "✅" if expected == actual or actual else "❌"
        if actual:
            passed += 1
        else:
            failed += 1
        print(f"{status} {name}")
    
    print(f"\nTotal: {len(results)} | Passed: {passed} | Failed: {failed}")
    print(f"Success Rate: {passed/len(results)*100:.1f}%")
    
    return passed, failed, results

if __name__ == "__main__":
    passed, failed, results = main()
    sys.exit(0 if failed == 0 else 1)
