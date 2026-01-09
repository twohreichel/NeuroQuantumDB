#!/usr/bin/env python3
"""Final Comprehensive API Test for VALIDATION_REPORT.md Update"""

import subprocess
import json
import sys
from datetime import datetime

API_KEY = "nqdb_03c495c620c646eaa7ce89dd2a78ce86"
BASE_URL = "http://127.0.0.1:8080"

results = {
    "working": [],
    "not_working": [],
    "partial": [],
    "test_details": []
}

def curl(method, endpoint, data=None, auth=True):
    cmd = ["curl", "-s", "-X", method]
    if auth:
        cmd.extend(["-H", f"X-API-Key: {API_KEY}"])
    if data:
        cmd.extend(["-H", "Content-Type: application/json", "-d", json.dumps(data)])
    cmd.append(f"{BASE_URL}{endpoint}")
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        return r.stdout
    except:
        return ""

def test(name, method, endpoint, data=None, auth=True, check_success=True):
    resp = curl(method, endpoint, data, auth)
    try:
        j = json.loads(resp) if resp else {}
        success = j.get("success", False)
        error = j.get("error")
        details = {
            "name": name,
            "endpoint": f"{method} {endpoint}",
            "success": success,
            "error": str(error)[:100] if error else None,
            "response_sample": str(j)[:200]
        }
        results["test_details"].append(details)
        
        if check_success:
            if success:
                results["working"].append(name)
                return True, j
            else:
                results["not_working"].append(name)
                return False, j
        return True, j
    except:
        results["test_details"].append({"name": name, "raw": resp[:200]})
        if "not configured" in resp:
            results["not_working"].append(f"{name} (not configured)")
        else:
            results["partial"].append(name)
        return False, {"raw": resp}

print("=" * 70)
print("NEUROQUANTUMDB FINAL VALIDATION TEST")
print(f"Timestamp: {datetime.now().isoformat()}")
print("=" * 70)

# 1. Health Check
print("\n[1] Health Check...")
success, data = test("Health Check", "GET", "/health", auth=False)
if success and data.get("data", {}).get("status") == "healthy":
    print(f"  ✅ Status: healthy, Version: {data.get('data', {}).get('version')}")
else:
    print(f"  ❌ Failed")

# 2. SQL Operations
print("\n[2] SQL Operations via /api/v1/query...")

ops = [
    ("SQL SELECT", {"query": "SELECT * FROM users LIMIT 5"}),
    ("SQL INSERT", {"query": "INSERT INTO users (id, name, email, age) VALUES (999, 'FinalTest', 'final@test.com', 35)"}),
    ("SQL UPDATE", {"query": "UPDATE users SET age = 36 WHERE name = 'FinalTest'"}),
    ("SQL SELECT WHERE", {"query": "SELECT * FROM users WHERE age > 30"}),
    ("SQL SELECT ORDER BY", {"query": "SELECT * FROM users ORDER BY age DESC"}),
    ("SQL SELECT LIMIT", {"query": "SELECT * FROM users LIMIT 3"}),
    ("SQL DELETE", {"query": "DELETE FROM users WHERE name = 'FinalTest'"}),
]

for name, payload in ops:
    success, data = test(name, "POST", "/api/v1/query", payload)
    print(f"  {'✅' if success else '❌'} {name}")

# 3. QSQL Functions (via SQL)
print("\n[3] QSQL Neuromorphic Functions (via SQL)...")
qsql_ops = [
    ("QSQL NEUROMATCH", {"query": "SELECT * FROM users WHERE NEUROMATCH(name, 'John') > 0.5"}),
    ("QSQL QUANTUM_SEARCH", {"query": "SELECT * FROM users WHERE QUANTUM_SEARCH(name, 'test')"}),
]
for name, payload in qsql_ops:
    success, data = test(name, "POST", "/api/v1/query", payload)
    print(f"  {'✅' if success else '❌'} {name}")

# 4. REST API Operations
print("\n[4] REST API Table Operations...")

# Create Table
success, _ = test("REST Create Table", "POST", "/api/v1/tables", {
    "schema": {
        "name": f"validation_test_{int(datetime.now().timestamp())}",
        "columns": [
            {"name": "id", "data_type": "Integer", "nullable": False, "primary_key": True},
            {"name": "data", "data_type": "Text", "nullable": True}
        ]
    },
    "if_not_exists": True
})
print(f"  {'✅' if success else '❌'} REST Create Table")

# Insert via REST
success, _ = test("REST Insert Data", "POST", "/api/v1/tables/users/data", {
    "table_name": "users",
    "records": [{"id": 998, "name": "RestValidation", "email": "validate@rest.com", "age": 27}]
})
print(f"  {'✅' if success else '❌'} REST Insert Data")

# Query via REST
success, _ = test("REST Query Data", "POST", "/api/v1/tables/users/query", {
    "table_name": "users",
    "limit": 5
})
print(f"  {'✅' if success else '❌'} REST Query Data")

# 5. Advanced Features
print("\n[5] Advanced Features (Specialized Endpoints)...")

endpoints = [
    ("DNA Compression", "POST", "/api/v1/dna/compress", {
        "sequences": ["ATCGATCGATCGATCG", "GCTAGCTAGCTAGCTA"],
        "algorithm": "KmerBased",
        "compression_level": 5
    }),
    ("Quantum Search", "POST", "/api/v1/quantum/search", {
        "table_name": "users",
        "query_vector": [0.5, 0.3, 0.2, 0.1],
        "similarity_threshold": 0.5,
        "max_results": 10
    }),
    ("Neural Train", "POST", "/api/v1/neural/train", {
        "network_name": "validation_network",
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
        }
    }),
    ("EEG List Users", "GET", "/api/v1/biometric/eeg/users", None),
    ("Performance Stats", "GET", "/api/v1/stats/performance", None),
]

for name, method, endpoint, data in endpoints:
    success, _ = test(name, method, endpoint, data)
    print(f"  {'✅' if success else '❌'} {name}")

# 6. Authentication & Security
print("\n[6] Authentication & Security...")

# Invalid API Key
resp = curl("POST", "/api/v1/query", {"query": "SELECT 1"}, auth=False)
if "unauthorized" in resp.lower() or "authentication" in resp.lower():
    results["working"].append("Unauthorized Request Rejection")
    print("  ✅ Unauthorized Request Rejection")
else:
    print("  ❌ Unauthorized Request Rejection")

# Login disabled
success, data = test("Login Disabled (501)", "POST", "/api/v1/auth/login", {"username": "x", "password": "x"}, auth=False, check_success=False)
if data.get("error", {}).get("NotImplemented") or "not implemented" in str(data).lower():
    results["working"].append("JWT Login Disabled (Security)")
    print("  ✅ JWT Login Disabled (Security)")
else:
    print("  ⚠️ JWT Login Status Unknown")

# WebSocket auth required
resp = curl("GET", "/ws", auth=False)
if "auth" in resp.lower():
    results["working"].append("WebSocket Auth Required")
    print("  ✅ WebSocket Auth Required")

# Summary
print("\n" + "=" * 70)
print("SUMMARY")
print("=" * 70)

print(f"\n✅ WORKING ({len(results['working'])}):")
for item in sorted(set(results["working"])):
    print(f"   • {item}")

print(f"\n❌ NOT WORKING ({len(results['not_working'])}):")
for item in sorted(set(results["not_working"])):
    print(f"   • {item}")

if results["partial"]:
    print(f"\n⚠️ PARTIAL/UNKNOWN ({len(results['partial'])}):")
    for item in sorted(set(results["partial"])):
        print(f"   • {item}")

total = len(results["working"]) + len(results["not_working"])
rate = len(results["working"]) / total * 100 if total > 0 else 0
print(f"\n📊 SUCCESS RATE: {rate:.1f}%")
print("=" * 70)

# Write JSON output
with open("validation_test_results.json", "w") as f:
    json.dump(results, f, indent=2)
print("\n📄 Detailed results saved to validation_test_results.json")
