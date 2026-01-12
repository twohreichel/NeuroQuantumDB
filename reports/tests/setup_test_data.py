#!/usr/bin/env python3
"""Setup test data for SQL function tests"""

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
        error = resp.get("error")
        if success:
            return True, resp
        else:
            return False, str(error)[:100] if error else "Unknown error"
    except Exception as e:
        return False, str(e)

print("="*60)
print("SETTING UP TEST DATA FOR NEUROQUANTUMDB")
print("="*60)

# Test users with all required columns
test_users = [
    (2, "John Doe", "john@example.com", 25),
    (3, "Jane Smith", "jane@example.com", 30),
    (4, "Bob Johnson", "bob@example.com", 35),
    (5, "Alice Williams", "alice@example.com", 28),
    (6, "Charlie Brown", "charlie@example.com", 45),
    (7, "TestUser1", "test1@example.com", 22),
    (8, "TestUser2", "test2@example.com", 38),
    (9, "TestUser3", "test3@example.com", 40),
    (10, "Test", "unique@test.com", 33),
    (11, "David Lee", "david@example.com", 55),
    (12, "Emily Chen", "emily@example.com", 19),
]

# Test orders with all required columns
test_orders = [
    (2, 1, 50.00, "AutoUser"),
    (3, 2, 150.00, "John Doe"),
    (4, 3, 200.00, "Jane Smith"),
    (5, 4, 75.00, "Bob Johnson"),
    (6, 5, 300.00, "Alice Williams"),
    (7, 2, 125.00, "John Doe"),
    (8, 3, 180.00, "Jane Smith"),
]

print("\n📊 Inserting test users...")
for uid, name, email, age in test_users:
    query = f"INSERT INTO users (id, name, email, age) VALUES ({uid}, '{name}', '{email}', {age})"
    success, result = run_sql(query)
    status = "✅" if success else "❌"
    print(f"  {status} User {uid}: {name}")

print("\n📊 Inserting test orders...")
for oid, user_id, amount, customer in test_orders:
    query = f"INSERT INTO orders (id, user_id, amount, customer) VALUES ({oid}, {user_id}, {amount}, '{customer}')"
    success, result = run_sql(query)
    status = "✅" if success else "❌"
    print(f"  {status} Order {oid}: {customer} - ${amount}")

# Verify data
print("\n📊 Verifying data...")
success, result = run_sql("SELECT COUNT(*) as count FROM users")
if success:
    rows = result.get("data", {}).get("rows", [])
    if rows:
        print(f"  ✅ Users table: {rows[0].get('count', 'unknown')} records")

success, result = run_sql("SELECT COUNT(*) as count FROM orders")
if success:
    rows = result.get("data", {}).get("rows", [])
    if rows:
        print(f"  ✅ Orders table: {rows[0].get('count', 'unknown')} records")

print("\n" + "="*60)
print("TEST DATA SETUP COMPLETE")
print("="*60)
