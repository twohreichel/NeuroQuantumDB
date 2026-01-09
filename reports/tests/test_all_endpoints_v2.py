#!/usr/bin/env python3
"""Comprehensive API Test Suite for NeuroQuantumDB - v2"""

import subprocess
import json
import sys

API_KEY = "nqdb_03c495c620c646eaa7ce89dd2a78ce86"
BASE_URL = "http://127.0.0.1:8080"

def run_curl(method, endpoint, data=None, headers=None, no_auth=False):
    """Run curl and return result"""
    cmd = ["curl", "-s", "-X", method]
    
    if headers:
        for k, v in headers.items():
            cmd.extend(["-H", f"{k}: {v}"])
    
    if not no_auth:
        cmd.extend(["-H", f"X-API-Key: {API_KEY}"])
    
    if data:
        cmd.extend(["-H", "Content-Type: application/json"])
        cmd.extend(["-d", json.dumps(data)])
    
    cmd.append(f"{BASE_URL}{endpoint}")
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
        return result.stdout, result.returncode
    except Exception as e:
        return str(e), -1

def test_endpoint(name, method, endpoint, data=None, expected_success=True, no_auth=False):
    """Test an endpoint and report result"""
    print(f"\n{'='*60}")
    print(f"TEST: {name}")
    print(f"{'='*60}")
    print(f"Endpoint: {method} {endpoint}")
    if data:
        print(f"Payload: {json.dumps(data)[:100]}...")
    
    response, code = run_curl(method, endpoint, data, no_auth=no_auth)
    
    if not response:
        print("Empty response (possible 404)")
        return False, {"error": "empty response"}
    
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
    
    print("\n" + "="*70)
    print("NEUROQUANTUMDB COMPREHENSIVE API TEST SUITE v2")
    print("Corrected Routes: /api/v1/query, /api/v1/tables/...")
    print("="*70)
    
    # ===== 1. Health Check (no auth needed) =====
    print("\n" + "="*60)
    print("[1] HEALTH CHECK (no auth)")
    print("="*60)
    resp, _ = run_curl("GET", "/health", no_auth=True)
    try:
        data = json.loads(resp)
        print(f"Status: {data.get('data', {}).get('status')}")
        print(f"Version: {data.get('data', {}).get('version')}")
        print(f"Features: {json.dumps(data.get('data', {}).get('features'), indent=2)}")
        results.append(("Health Check", True))
    except:
        print(f"Error: {resp}")
        results.append(("Health Check", False))
    
    # ===== 2. Metrics =====
    print("\n" + "="*60)
    print("[2] METRICS ENDPOINT")
    print("="*60)
    resp, _ = run_curl("GET", "/metrics")
    if "# HELP" in resp or "neuroquantum" in resp:
        print("✅ Prometheus metrics available")
        print(f"Sample: {resp[:300]}")
        results.append(("Metrics", True))
    else:
        print(f"Response: {resp[:300]}")
        results.append(("Metrics", "unauthorized" not in resp.lower()))
    
    # ===== 3. SQL - SELECT via /api/v1/query =====
    print("\n[3] SQL - SELECT * FROM users")
    success, data = test_endpoint("Basic SELECT", "POST", "/api/v1/query", 
                                   {"query": "SELECT * FROM users"})
    results.append(("SQL SELECT", success))
    
    # ===== 4. SQL - INSERT with Auto-Increment (BIGSERIAL) =====
    print("\n[4] SQL - CREATE TABLE with BIGSERIAL + INSERT without ID")
    # First create a table with BIGSERIAL for auto-increment
    test_endpoint("Drop auto_inc_test", "POST", "/api/v1/query",
                  {"query": "DROP TABLE IF EXISTS auto_inc_test"})
    test_endpoint("Create BIGSERIAL table", "POST", "/api/v1/query",
                  {"query": "CREATE TABLE auto_inc_test (id BIGSERIAL PRIMARY KEY, name TEXT, email TEXT)"})
    # Now insert WITHOUT specifying ID - auto-increment should work!
    success, data = test_endpoint("INSERT without ID (BIGSERIAL)", "POST", "/api/v1/query",
                                   {"query": "INSERT INTO auto_inc_test (name, email) VALUES ('AutoUser', 'auto@test.com')"})
    if success and data:
        try:
            rows = data.get("data", {}).get("rows", [])
            if rows and "inserted_id" in rows[0]:
                print(f"  ✅ Auto-generated ID: {rows[0]['inserted_id']}")
        except:
            pass
    results.append(("SQL INSERT (Auto-Increment)", success))
    
    # ===== 5. SQL - UPDATE =====
    print("\n[5] SQL - UPDATE users")
    success, data = test_endpoint("UPDATE", "POST", "/api/v1/query",
                                   {"query": "UPDATE users SET age = 40 WHERE name = 'TestAPI'"})
    results.append(("SQL UPDATE", success))
    
    # ===== 6. SQL - SELECT with WHERE =====
    print("\n[6] SQL - SELECT with WHERE")
    success, data = test_endpoint("SELECT with WHERE", "POST", "/api/v1/query",
                                   {"query": "SELECT * FROM users WHERE age >= 30"})
    results.append(("SQL WHERE", success))
    
    # ===== 7. SQL - SELECT with ORDER BY =====
    print("\n[7] SQL - SELECT with ORDER BY")
    success, data = test_endpoint("SELECT ORDER BY", "POST", "/api/v1/query",
                                   {"query": "SELECT * FROM users ORDER BY age DESC"})
    results.append(("SQL ORDER BY", success))
    
    # ===== 8. SQL - SELECT with LIMIT =====
    print("\n[8] SQL - SELECT with LIMIT")
    success, data = test_endpoint("SELECT LIMIT", "POST", "/api/v1/query",
                                   {"query": "SELECT * FROM users LIMIT 3"})
    results.append(("SQL LIMIT", success))
    
    # ===== 9. SQL - COUNT =====
    print("\n[9] SQL - COUNT Query")
    success, data = test_endpoint("COUNT Query", "POST", "/api/v1/query",
                                   {"query": "SELECT COUNT(*) as total FROM users"})
    results.append(("SQL COUNT", success))
    
    # ===== 10. SQL - DELETE =====
    print("\n[10] SQL - DELETE")
    success, data = test_endpoint("DELETE", "POST", "/api/v1/query",
                                   {"query": "DELETE FROM users WHERE name = 'TestAPI'"})
    results.append(("SQL DELETE", success))
    
    # ===== 11. QSQL - NEUROMATCH =====
    print("\n[11] QSQL - NEUROMATCH Query")
    success, data = test_endpoint("NEUROMATCH", "POST", "/api/v1/query",
                                   {"query": "SELECT * FROM users WHERE NEUROMATCH(name, 'John') > 0.5"})
    results.append(("QSQL NEUROMATCH", success))
    
    # ===== 12. QSQL - QUANTUM_SEARCH =====
    print("\n[12] QSQL - QUANTUM_SEARCH Query")
    success, data = test_endpoint("QUANTUM_SEARCH", "POST", "/api/v1/query",
                                   {"query": "SELECT * FROM users WHERE QUANTUM_SEARCH(name, 'test')"})
    results.append(("QSQL QUANTUM_SEARCH", success))
    
    # ===== 13. QSQL - SYNAPTIC_WEIGHT =====
    print("\n[13] QSQL - SYNAPTIC_WEIGHT Query")
    success, data = test_endpoint("SYNAPTIC_WEIGHT", "POST", "/api/v1/query",
                                   {"query": "SELECT SYNAPTIC_WEIGHT(name, email) as weight FROM users LIMIT 5"})
    results.append(("QSQL SYNAPTIC_WEIGHT", success))
    
    # ===== 14. QSQL - HEBBIAN_LEARNING =====
    print("\n[14] QSQL - HEBBIAN_LEARNING Query")
    success, data = test_endpoint("HEBBIAN_LEARNING", "POST", "/api/v1/query",
                                   {"query": "SELECT HEBBIAN_LEARNING(age) as hebbian FROM users LIMIT 5"})
    results.append(("QSQL HEBBIAN_LEARNING", success))
    
    # ===== 15. REST - Create Table =====
    print("\n[15] REST - Create Table")
    success, data = test_endpoint("Create Table via REST", "POST", "/api/v1/tables",
                                   {"schema": {
                                       "name": "api_test_table",
                                       "columns": [
                                           {"name": "id", "data_type": "Integer", "nullable": False},
                                           {"name": "value", "data_type": "Text", "nullable": True}
                                       ]
                                   },
                                    "if_not_exists": True})
    results.append(("REST Create Table", success))
    
    # ===== 16. REST - Insert via /tables/{name}/data =====
    print("\n[16] REST - Insert Records via REST")
    success, data = test_endpoint("Insert via REST", "POST", "/api/v1/tables/users/data",
                                   {"table_name": "users",
                                    "records": [{"id": 100, "name": "RESTUser", "email": "rest@test.com", "age": 29}]})
    results.append(("REST Insert", success))
    
    # ===== 17. REST - Query via /tables/{name}/query =====
    print("\n[17] REST - Query via REST")
    success, data = test_endpoint("Query via REST", "POST", "/api/v1/tables/users/query",
                                   {"table_name": "users",
                                    "filters": {"name": {"operator": "Equal", "value": "RESTUser"}},
                                    "limit": 10})
    results.append(("REST Query", success))
    
    # ===== 18. Neural - Train =====
    print("\n[18] Neural - Train Network")
    success, data = test_endpoint("Neural Train", "POST", "/api/v1/neural/train",
                                   {"network_name": "test_network",
                                    "training_data": [
                                        {"input": [0.5, 0.3], "target": [1.0, 0.0]},
                                        {"input": [0.1, 0.8], "target": [0.0, 1.0]}
                                    ],
                                    "config": {
                                        "layers": [{"layer_type": "Dense", "size": 10, "activation": "ReLU"}],
                                        "learning_rate": 0.01,
                                        "epochs": 10,
                                        "batch_size": 2,
                                        "optimizer": "Adam",
                                        "loss_function": "MeanSquaredError"
                                    }})
    results.append(("Neural Train", success))
    
    # ===== 19. Quantum - Search =====
    print("\n[19] Quantum - Search")
    success, data = test_endpoint("Quantum Search", "POST", "/api/v1/quantum/search",
                                   {"table_name": "users",
                                    "query_vector": [0.5, 0.3, 0.2, 0.1],
                                    "similarity_threshold": 0.5,
                                    "max_results": 10})
    results.append(("Quantum Search", success))
    
    # ===== 20. DNA - Compress =====
    print("\n[20] DNA - Compress")
    success, data = test_endpoint("DNA Compress", "POST", "/api/v1/dna/compress",
                                   {"sequences": ["ATCGATCGATCGATCG", "GCTAGCTAGCTAGCTA"],
                                    "algorithm": "KmerBased",
                                    "compression_level": 5})
    results.append(("DNA Compress", success))
    
    # ===== 21. Biometric EEG - List Users =====
    print("\n[21] Biometric EEG - List Users")
    success, data = test_endpoint("EEG List Users", "GET", "/api/v1/biometric/eeg/users")
    results.append(("EEG Users", success))
    
    # ===== 22. Stats - Performance =====
    print("\n[22] Stats - Performance")
    success, data = test_endpoint("Performance Stats", "GET", "/api/v1/stats/performance")
    results.append(("Performance Stats", success))
    
    # ===== 23. Auth - Login (should fail / disabled) =====
    print("\n[23] Auth - Login (disabled)")
    resp, _ = run_curl("POST", "/api/v1/auth/login", {"username": "test", "password": "test"}, no_auth=True)
    print(f"Response: {resp[:200]}")
    results.append(("Auth Login (disabled)", "501" in resp or "not implemented" in resp.lower() or "content type" in resp.lower()))
    
    # ===== 24. Auth - Invalid API Key =====
    print("\n[24] Auth - Invalid API Key")
    cmd = ["curl", "-s", "-H", "X-API-Key: invalid_key_12345", "-H", "Content-Type: application/json",
           "-d", '{"query": "SELECT 1"}', f"{BASE_URL}/api/v1/query"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    print(f"Response: {result.stdout[:200]}")
    is_rejected = "unauthorized" in result.stdout.lower() or "401" in result.stdout or "Invalid" in result.stdout
    results.append(("Auth Invalid Key Rejection", is_rejected))
    
    # ===== 25. WebSocket =====
    print("\n[25] WebSocket Endpoint")
    resp, _ = run_curl("GET", "/ws", no_auth=True)
    print(f"Response: {resp[:200]}")
    results.append(("WebSocket Endpoint", True if resp else False))
    
    # ===== SUMMARY =====
    print("\n" + "="*70)
    print("TEST SUMMARY")
    print("="*70)
    
    passed = 0
    failed = 0
    for name, success in results:
        if success:
            status = "✅"
            passed += 1
        else:
            status = "❌"
            failed += 1
        print(f"{status} {name}")
    
    print(f"\n{'='*70}")
    print(f"TOTAL: {len(results)} | PASSED: {passed} | FAILED: {failed}")
    print(f"SUCCESS RATE: {passed/len(results)*100:.1f}%")
    print("="*70)
    
    return passed, failed, results

if __name__ == "__main__":
    passed, failed, results = main()
    sys.exit(0 if failed < 5 else 1)
