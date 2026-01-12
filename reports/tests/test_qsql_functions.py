#!/usr/bin/env python3
"""Test additional QSQL neuromorphic functions"""

import subprocess
import json

API_KEY = "nqdb_03c495c620c646eaa7ce89dd2a78ce86"
BASE_URL = "http://127.0.0.1:8080"

def run_sql(query):
    """Execute SQL query via API"""
    cmd = ["curl", "-s", "-X", "POST",
           "-H", f"X-API-Key: {API_KEY}",
           "-H", "Content-Type: application/json",
           "-d", json.dumps({"query": query}),
           f"{BASE_URL}/api/v1/query"]
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        resp = json.loads(r.stdout) if r.stdout else {}
        success = resp.get("success", False)
        return success, resp
    except Exception as e:
        return False, {"error": str(e)}

print("="*70)
print("NEUROQUANTUMDB - QSQL NEUROMORPHE FUNKTIONEN TEST")
print("="*70)

qsql_tests = [
    # NEUROMATCH Tests
    ("NEUROMATCH basic", "SELECT * FROM users WHERE NEUROMATCH(name, 'John') > 0.5 LIMIT 5"),
    ("NEUROMATCH low threshold", "SELECT * FROM users WHERE NEUROMATCH(name, 'Test') > 0.3 LIMIT 5"),
    ("NEUROMATCH on email", "SELECT * FROM users WHERE NEUROMATCH(email, 'example') > 0.5 LIMIT 5"),
    ("NEUROMATCH combined with AND", "SELECT * FROM users WHERE NEUROMATCH(name, 'User') > 0.4 AND age > 20 LIMIT 5"),
    ("NEUROMATCH with ORDER BY", "SELECT * FROM users WHERE NEUROMATCH(name, 'John') > 0.3 ORDER BY name LIMIT 5"),
    
    # QUANTUM_SEARCH Tests
    ("QUANTUM_SEARCH on name", "SELECT * FROM users WHERE QUANTUM_SEARCH(name, 'test')"),
    ("QUANTUM_SEARCH on email", "SELECT * FROM users WHERE QUANTUM_SEARCH(email, 'example')"),
    ("QUANTUM_SEARCH with LIMIT", "SELECT * FROM users WHERE QUANTUM_SEARCH(name, 'User') LIMIT 3"),
    
    # HEBBIAN_LEARNING Tests
    ("HEBBIAN_LEARNING on age", "SELECT name, HEBBIAN_LEARNING(age) FROM users LIMIT 5"),
    ("HEBBIAN_LEARNING in SELECT", "SELECT HEBBIAN_LEARNING(age) as learning FROM users WHERE age > 25 LIMIT 3"),
    
    # JOIN with QSQL
    ("NEUROMATCH in JOIN", "SELECT u.name, o.amount FROM users u INNER JOIN orders o ON u.id = o.user_id WHERE NEUROMATCH(u.name, 'Test') > 0.3 LIMIT 5"),
    
    # EXPLAIN for neuromorphic queries
    ("EXPLAIN NEUROMATCH", "EXPLAIN SELECT * FROM users WHERE NEUROMATCH(name, 'John') > 0.5"),
    ("EXPLAIN QUANTUM_SEARCH", "EXPLAIN SELECT * FROM users WHERE QUANTUM_SEARCH(name, 'test')"),
    
    # Aggregate with QSQL
    ("COUNT with NEUROMATCH", "SELECT COUNT(*) as matches FROM users WHERE NEUROMATCH(name, 'User') > 0.3"),
    
    # Complex queries
    ("QSQL with subquery", "SELECT * FROM users WHERE id IN (SELECT user_id FROM orders) AND NEUROMATCH(name, 'Test') > 0.3"),
]

results = {"working": [], "not_working": []}

for name, query in qsql_tests:
    success, resp = run_sql(query)
    if success:
        rows = resp.get("data", {}).get("rows", [])
        row_count = len(rows) if rows else 0
        results["working"].append(name)
        print(f"✅ {name}: {row_count} rows")
    else:
        error = resp.get("error", {})
        error_msg = str(error)[:60] if error else "Unknown error"
        results["not_working"].append(f"{name}: {error_msg}")
        print(f"❌ {name}: {error_msg}")

print("\n" + "="*70)
print("ZUSAMMENFASSUNG")
print("="*70)
print(f"\n✅ Funktioniert: {len(results['working'])}")
print(f"❌ Funktioniert nicht: {len(results['not_working'])}")

if results['not_working']:
    print("\n--- Nicht funktionierende QSQL-Features ---")
    for item in results['not_working']:
        print(f"  ❌ {item}")

success_rate = len(results['working']) / (len(results['working']) + len(results['not_working'])) * 100
print(f"\n📊 Erfolgsrate: {success_rate:.1f}%")
