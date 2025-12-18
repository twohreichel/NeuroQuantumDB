#!/usr/bin/env python3
"""Test aller gängigen SQL-Funktionen für NeuroQuantumDB"""

import subprocess
import json

API_KEY = "nqdb_03c495c620c646eaa7ce89dd2a78ce86"
BASE_URL = "http://127.0.0.1:8080"

def test_sql(name, query):
    """Test a SQL query and return result"""
    cmd = ["curl", "-s", "-X", "POST",
           "-H", f"X-API-Key: {API_KEY}",
           "-H", "Content-Type: application/json",
           "-d", json.dumps({"query": query}),
           f"{BASE_URL}/api/v1/query"]
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        resp = r.stdout
        try:
            j = json.loads(resp) if resp else {}
            success = j.get("success", False)
            error = j.get("error")
            if success:
                rows = j.get("data", {}).get("rows_affected", 0)
                return "✅", f"OK ({rows} rows)"
            else:
                err_msg = str(error)[:80] if error else "Unknown error"
                return "❌", err_msg
        except:
            return "❌", resp[:80] if resp else "Empty response"
    except Exception as e:
        return "❌", str(e)[:80]

print("="*80)
print("NEUROQUANTUMDB - UMFASSENDER SQL-FUNKTIONSTEST")
print("="*80)

# Kategorien von SQL-Tests
tests = {
    "Basis SELECT": [
        ("SELECT *", "SELECT * FROM users LIMIT 5"),
        ("SELECT Spalten", "SELECT name, email FROM users LIMIT 5"),
        ("SELECT mit Alias", "SELECT name AS username FROM users LIMIT 5"),
    ],
    "WHERE Klauseln": [
        ("WHERE =", "SELECT * FROM users WHERE id = 1"),
        ("WHERE >", "SELECT * FROM users WHERE age > 25"),
        ("WHERE <", "SELECT * FROM users WHERE age < 50"),
        ("WHERE >=", "SELECT * FROM users WHERE age >= 30"),
        ("WHERE <=", "SELECT * FROM users WHERE age <= 40"),
        ("WHERE <>", "SELECT * FROM users WHERE age <> 30"),
        ("WHERE !=", "SELECT * FROM users WHERE age != 30"),
        ("WHERE AND", "SELECT * FROM users WHERE age > 20 AND age < 50"),
        ("WHERE OR", "SELECT * FROM users WHERE age < 20 OR age > 50"),
        ("WHERE NOT", "SELECT * FROM users WHERE NOT age = 30"),
        ("WHERE IN", "SELECT * FROM users WHERE age IN (25, 30, 35)"),
        ("WHERE NOT IN", "SELECT * FROM users WHERE age NOT IN (25, 30)"),
        ("WHERE BETWEEN", "SELECT * FROM users WHERE age BETWEEN 20 AND 40"),
        ("WHERE IS NULL", "SELECT * FROM users WHERE email IS NULL"),
        ("WHERE IS NOT NULL", "SELECT * FROM users WHERE email IS NOT NULL"),
    ],
    "LIKE & Pattern Matching": [
        ("LIKE %pattern%", "SELECT * FROM users WHERE name LIKE '%test%'"),
        ("LIKE pattern%", "SELECT * FROM users WHERE name LIKE 'Test%'"),
        ("LIKE %pattern", "SELECT * FROM users WHERE name LIKE '%User'"),
        ("LIKE mit _", "SELECT * FROM users WHERE name LIKE 'Test_'"),
        ("NOT LIKE", "SELECT * FROM users WHERE name NOT LIKE '%test%'"),
        ("ILIKE (case-insensitive)", "SELECT * FROM users WHERE name ILIKE '%TEST%'"),
    ],
    "Aggregatfunktionen": [
        ("COUNT(*)", "SELECT COUNT(*) FROM users"),
        ("COUNT(column)", "SELECT COUNT(name) FROM users"),
        ("COUNT DISTINCT", "SELECT COUNT(DISTINCT name) FROM users"),
        ("SUM", "SELECT SUM(age) FROM users"),
        ("AVG", "SELECT AVG(age) FROM users"),
        ("MIN", "SELECT MIN(age) FROM users"),
        ("MAX", "SELECT MAX(age) FROM users"),
        ("COUNT mit AS", "SELECT COUNT(*) AS total FROM users"),
    ],
    "GROUP BY & HAVING": [
        ("GROUP BY", "SELECT name, COUNT(*) FROM users GROUP BY name"),
        ("GROUP BY mit HAVING", "SELECT name, COUNT(*) FROM users GROUP BY name HAVING COUNT(*) > 1"),
        ("GROUP BY mehrere Spalten", "SELECT name, email, COUNT(*) FROM users GROUP BY name, email"),
    ],
    "ORDER BY": [
        ("ORDER BY ASC", "SELECT * FROM users ORDER BY name ASC"),
        ("ORDER BY DESC", "SELECT * FROM users ORDER BY age DESC"),
        ("ORDER BY mehrere", "SELECT * FROM users ORDER BY name ASC, age DESC"),
        ("ORDER BY mit LIMIT", "SELECT * FROM users ORDER BY age DESC LIMIT 3"),
    ],
    "LIMIT & OFFSET": [
        ("LIMIT", "SELECT * FROM users LIMIT 5"),
        ("LIMIT OFFSET", "SELECT * FROM users LIMIT 5 OFFSET 2"),
        ("OFFSET ohne LIMIT", "SELECT * FROM users OFFSET 2"),
    ],
    "DISTINCT": [
        ("DISTINCT", "SELECT DISTINCT name FROM users"),
        ("DISTINCT mehrere Spalten", "SELECT DISTINCT name, email FROM users"),
    ],
    "JOINs": [
        ("INNER JOIN", "SELECT u.name, o.amount FROM users u INNER JOIN orders o ON u.id = o.user_id"),
        ("LEFT JOIN", "SELECT u.name, o.amount FROM users u LEFT JOIN orders o ON u.id = o.user_id"),
        ("RIGHT JOIN", "SELECT u.name, o.amount FROM users u RIGHT JOIN orders o ON u.id = o.user_id"),
        ("FULL OUTER JOIN", "SELECT u.name, o.amount FROM users u FULL OUTER JOIN orders o ON u.id = o.user_id"),
        ("CROSS JOIN", "SELECT u.name, o.amount FROM users u CROSS JOIN orders o"),
        ("Self JOIN", "SELECT a.name, b.name FROM users a, users b WHERE a.id != b.id"),
        ("JOIN mit WHERE", "SELECT u.name FROM users u JOIN orders o ON u.id = o.user_id WHERE o.amount > 100"),
    ],
    "Subqueries": [
        ("Subquery in WHERE", "SELECT * FROM users WHERE id IN (SELECT user_id FROM orders)"),
        ("Subquery in FROM", "SELECT * FROM (SELECT name FROM users) AS subq"),
        ("Subquery mit EXISTS", "SELECT * FROM users u WHERE EXISTS (SELECT 1 FROM orders WHERE user_id = u.id)"),
        ("Correlated Subquery", "SELECT * FROM users u WHERE age > (SELECT AVG(age) FROM users)"),
    ],
    "UNION & Mengenoperationen": [
        ("UNION", "SELECT name FROM users UNION SELECT customer FROM orders"),
        ("UNION ALL", "SELECT name FROM users UNION ALL SELECT customer FROM orders"),
        ("INTERSECT", "SELECT name FROM users INTERSECT SELECT customer FROM orders"),
        ("EXCEPT", "SELECT name FROM users EXCEPT SELECT customer FROM orders"),
    ],
    "INSERT Varianten": [
        ("INSERT VALUES", "INSERT INTO users (name, email, age) VALUES ('SQLTest', 'sql@test.com', 25)"),
        ("INSERT mehrere Zeilen", "INSERT INTO users (name, email) VALUES ('Test1', 'a@b.com'), ('Test2', 'c@d.com')"),
        ("INSERT DEFAULT", "INSERT INTO users (name) VALUES ('DefaultTest')"),
    ],
    "UPDATE Varianten": [
        ("UPDATE mit WHERE", "UPDATE users SET age = 99 WHERE name = 'SQLTest'"),
        ("UPDATE mehrere Spalten", "UPDATE users SET name = 'Updated', age = 50 WHERE name = 'SQLTest'"),
        ("UPDATE ohne WHERE", "UPDATE users SET age = age + 1"),
    ],
    "DELETE Varianten": [
        ("DELETE mit WHERE", "DELETE FROM users WHERE name = 'SQLTest'"),
        ("DELETE mit LIKE", "DELETE FROM users WHERE name LIKE 'Test%'"),
        ("DELETE mit IN", "DELETE FROM users WHERE name IN ('Test1', 'Test2', 'DefaultTest')"),
    ],
    "DDL Statements": [
        ("CREATE TABLE", "CREATE TABLE test_ddl (id INTEGER PRIMARY KEY, name TEXT)"),
        ("DROP TABLE", "DROP TABLE test_ddl"),
        ("ALTER TABLE ADD", "ALTER TABLE users ADD COLUMN status TEXT"),
        ("ALTER TABLE DROP", "ALTER TABLE users DROP COLUMN status"),
        ("CREATE INDEX", "CREATE INDEX idx_name ON users(name)"),
        ("DROP INDEX", "DROP INDEX idx_name"),
        ("TRUNCATE", "TRUNCATE TABLE test_ddl"),
    ],
    "Transaktionskontrolle": [
        ("BEGIN", "BEGIN"),
        ("COMMIT", "COMMIT"),
        ("ROLLBACK", "ROLLBACK"),
        ("SAVEPOINT", "SAVEPOINT sp1"),
        ("ROLLBACK TO", "ROLLBACK TO sp1"),
    ],
    "CASE Expressions": [
        ("CASE WHEN", "SELECT name, CASE WHEN age > 30 THEN 'Senior' ELSE 'Junior' END FROM users"),
        ("CASE mit mehreren WHEN", "SELECT name, CASE WHEN age < 20 THEN 'Teen' WHEN age < 40 THEN 'Adult' ELSE 'Senior' END FROM users"),
    ],
    "String Funktionen": [
        ("UPPER", "SELECT UPPER(name) FROM users"),
        ("LOWER", "SELECT LOWER(name) FROM users"),
        ("LENGTH", "SELECT LENGTH(name) FROM users"),
        ("CONCAT", "SELECT CONCAT(name, ' - ', email) FROM users"),
        ("SUBSTRING", "SELECT SUBSTRING(name, 1, 3) FROM users"),
        ("TRIM", "SELECT TRIM(name) FROM users"),
        ("REPLACE", "SELECT REPLACE(name, 'Test', 'User') FROM users"),
    ],
    "Mathematische Funktionen": [
        ("ABS", "SELECT ABS(age) FROM users"),
        ("ROUND", "SELECT ROUND(age / 3.0, 2) FROM users"),
        ("CEIL/CEILING", "SELECT CEIL(age / 3.0) FROM users"),
        ("FLOOR", "SELECT FLOOR(age / 3.0) FROM users"),
        ("MOD", "SELECT MOD(age, 10) FROM users"),
        ("POWER", "SELECT POWER(age, 2) FROM users"),
        ("SQRT", "SELECT SQRT(age) FROM users"),
    ],
    "Datum/Zeit Funktionen": [
        ("CURRENT_DATE", "SELECT CURRENT_DATE"),
        ("CURRENT_TIME", "SELECT CURRENT_TIME"),
        ("CURRENT_TIMESTAMP", "SELECT CURRENT_TIMESTAMP"),
        ("NOW()", "SELECT NOW()"),
        ("DATE_ADD", "SELECT DATE_ADD(CURRENT_DATE, INTERVAL 1 DAY)"),
        ("DATE_SUB", "SELECT DATE_SUB(CURRENT_DATE, INTERVAL 1 DAY)"),
        ("EXTRACT", "SELECT EXTRACT(YEAR FROM CURRENT_DATE)"),
    ],
    "NULL Handling": [
        ("COALESCE", "SELECT COALESCE(email, 'no-email') FROM users"),
        ("NULLIF", "SELECT NULLIF(age, 0) FROM users"),
        ("IFNULL", "SELECT IFNULL(email, 'default') FROM users"),
    ],
    "Window Functions": [
        ("ROW_NUMBER", "SELECT name, ROW_NUMBER() OVER (ORDER BY age) FROM users"),
        ("RANK", "SELECT name, RANK() OVER (ORDER BY age) FROM users"),
        ("DENSE_RANK", "SELECT name, DENSE_RANK() OVER (ORDER BY age) FROM users"),
        ("LAG", "SELECT name, LAG(age) OVER (ORDER BY id) FROM users"),
        ("LEAD", "SELECT name, LEAD(age) OVER (ORDER BY id) FROM users"),
        ("SUM OVER", "SELECT name, SUM(age) OVER (PARTITION BY name) FROM users"),
    ],
    "CTE (Common Table Expressions)": [
        ("WITH ... AS", "WITH active_users AS (SELECT * FROM users WHERE age > 20) SELECT * FROM active_users"),
        ("Rekursives CTE", "WITH RECURSIVE cte AS (SELECT 1 AS n UNION ALL SELECT n+1 FROM cte WHERE n < 5) SELECT * FROM cte"),
    ],
}

# Ergebnisse sammeln
results = {"working": [], "not_working": [], "categories": {}}

for category, queries in tests.items():
    print(f"\n{'='*60}")
    print(f"📂 {category}")
    print(f"{'='*60}")
    
    category_results = []
    for name, query in queries:
        status, msg = test_sql(name, query)
        print(f"  {status} {name}: {msg[:50]}")
        
        if status == "✅":
            results["working"].append(f"{category}: {name}")
        else:
            results["not_working"].append(f"{category}: {name}")
        
        category_results.append({"name": name, "query": query, "status": status, "message": msg})
    
    results["categories"][category] = category_results

# Zusammenfassung
print("\n" + "="*80)
print("ZUSAMMENFASSUNG")
print("="*80)

print(f"\n✅ Funktioniert: {len(results['working'])}")
print(f"❌ Funktioniert nicht: {len(results['not_working'])}")
print(f"📊 Erfolgsrate: {len(results['working']) / (len(results['working']) + len(results['not_working'])) * 100:.1f}%")

print("\n--- Nicht funktionierende SQL-Features ---")
for item in results["not_working"]:
    print(f"  ❌ {item}")

# Speichern
with open("sql_test_results.json", "w") as f:
    json.dump(results, f, indent=2)

print("\n📄 Ergebnisse gespeichert in sql_test_results.json")
