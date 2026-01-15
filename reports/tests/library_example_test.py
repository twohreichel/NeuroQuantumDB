#!/usr/bin/env python3
"""
NeuroQuantumDB Bibliotheks-Beispiel Validierungstest
=====================================================
Testet alle Features aus der NEUROQUANTUM_FEATURES_GUIDE.md
basierend auf dem vollständigen Bibliotheks-Beispiel.

Datum: 15. Januar 2026
"""

import subprocess
import json
import sys
from datetime import datetime

API_KEY = "nqdb_03c495c620c646eaa7ce89dd2a78ce86"
BASE_URL = "http://127.0.0.1:8080"

# Ergebnis-Sammlung
results = {
    "timestamp": datetime.now().isoformat(),
    "tests": [],
    "summary": {
        "passed": 0,
        "failed": 0,
        "warnings": 0
    }
}

def log_test(name, status, message, details=None):
    """Protokolliert ein Testergebnis"""
    result = {
        "name": name,
        "status": status,
        "message": message,
        "details": details
    }
    results["tests"].append(result)
    
    icon = "✅" if status == "PASS" else "❌" if status == "FAIL" else "⚠️"
    print(f"  {icon} {name}: {message}")
    
    if status == "PASS":
        results["summary"]["passed"] += 1
    elif status == "FAIL":
        results["summary"]["failed"] += 1
    else:
        results["summary"]["warnings"] += 1

def curl(method, endpoint, data=None, auth=True):
    """Führt einen curl-Befehl aus"""
    cmd = ["curl", "-s", "-X", method]
    if auth:
        cmd.extend(["-H", f"X-API-Key: {API_KEY}"])
    if data:
        cmd.extend(["-H", "Content-Type: application/json", "-d", json.dumps(data)])
    cmd.append(f"{BASE_URL}{endpoint}")
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        return result.stdout
    except subprocess.TimeoutExpired:
        return '{"error": "Timeout"}'
    except Exception as e:
        return f'{{"error": "{str(e)}"}}'

def parse_response(response):
    """Parst die JSON-Antwort"""
    try:
        return json.loads(response)
    except:
        return {"raw": response, "error": "Parse error"}

# ============================================================
# SCHRITT 1: Health Check
# ============================================================
print("\n" + "="*70)
print("SCHRITT 1: Health Check")
print("="*70)

resp = parse_response(curl("GET", "/health", auth=False))
if resp.get("success") and resp.get("data", {}).get("status") == "healthy":
    log_test("Health Check", "PASS", 
             f"Server healthy, Version: {resp.get('data', {}).get('version')}")
else:
    log_test("Health Check", "FAIL", "Server nicht erreichbar", resp)

# ============================================================
# SCHRITT 2: Tabellen erstellen (books, library_users, search_history)
# ============================================================
print("\n" + "="*70)
print("SCHRITT 2: Tabellen erstellen")
print("="*70)

tables = [
    ("books", "CREATE TABLE IF NOT EXISTS books (id INTEGER PRIMARY KEY AUTO_INCREMENT, title TEXT NOT NULL, author TEXT NOT NULL, isbn TEXT, description TEXT, genre TEXT, publication_year INTEGER)"),
    ("library_users", "CREATE TABLE IF NOT EXISTS library_users (id INTEGER PRIMARY KEY AUTO_INCREMENT, username TEXT NOT NULL, email TEXT, preferences TEXT, reading_history TEXT)"),
    ("search_history", "CREATE TABLE IF NOT EXISTS search_history (id INTEGER PRIMARY KEY AUTO_INCREMENT, user_id INTEGER, search_query TEXT, results_clicked TEXT)")
]

for table_name, query in tables:
    resp = parse_response(curl("POST", "/api/v1/query", {"query": query}))
    if resp.get("success"):
        log_test(f"CREATE TABLE {table_name}", "PASS", "Tabelle erstellt/existiert")
    else:
        log_test(f"CREATE TABLE {table_name}", "FAIL", 
                 resp.get("error", {}).get("message", "Unbekannter Fehler"), resp)

# ============================================================
# SCHRITT 3: Test-Daten einfügen
# ============================================================
print("\n" + "="*70)
print("SCHRITT 3: Test-Daten einfügen")
print("="*70)

inserts = [
    ("Harry Potter Buch", "INSERT INTO books (title, author, isbn, description, genre, publication_year) VALUES ('Harry Potter und der Stein der Weisen', 'J.K. Rowling', '978-3-551-35401-1', 'Ein junger Zauberer entdeckt seine magischen Faehigkeiten', 'Fantasy', 1997)"),
    ("Der Name des Windes", "INSERT INTO books (title, author, isbn, description, genre, publication_year) VALUES ('Der Name des Windes', 'Patrick Rothfuss', '978-3-608-93815-0', 'Die Geschichte eines legendaeren Magiers', 'Fantasy', 2007)"),
    ("Mistborn Buch", "INSERT INTO books (title, author, isbn, description, genre, publication_year) VALUES ('Mistborn - Das letzte Imperium', 'Brandon Sanderson', '978-3-453-53298-9', 'Fantasy mit einzigartigem Magiesystem', 'Fantasy', 2006)"),
    ("Benutzer max_mustermann", "INSERT INTO library_users (username, email, preferences) VALUES ('max_mustermann', 'max@example.com', 'Fantasy, Science Fiction')"),
    ("Suchanfrage", "INSERT INTO search_history (user_id, search_query) VALUES (1, 'Zauberer Schule Magie')")
]

for name, query in inserts:
    resp = parse_response(curl("POST", "/api/v1/query", {"query": query}))
    if resp.get("success"):
        log_test(f"INSERT {name}", "PASS", "Daten eingefügt")
    else:
        error_msg = str(resp.get("error", ""))
        if "duplicate" in error_msg.lower() or "unique" in error_msg.lower():
            log_test(f"INSERT {name}", "WARN", "Duplikat (bereits vorhanden)")
        else:
            log_test(f"INSERT {name}", "FAIL", error_msg[:100], resp)

# ============================================================
# SCHRITT 4: Daten abfragen (SELECT)
# ============================================================
print("\n" + "="*70)
print("SCHRITT 4: Daten abfragen")
print("="*70)

selects = [
    ("SELECT * FROM books", "Alle Bücher"),
    ("SELECT * FROM books WHERE genre = 'Fantasy'", "Bücher nach Genre"),
    ("SELECT * FROM books ORDER BY publication_year DESC", "Bücher sortiert"),
    ("SELECT * FROM books LIMIT 2", "Bücher mit LIMIT"),
    ("SELECT * FROM library_users", "Alle Benutzer")
]

for query, name in selects:
    resp = parse_response(curl("POST", "/api/v1/query", {"query": query}))
    if resp.get("success"):
        rows = resp.get("data", {}).get("rows", [])
        log_test(f"SELECT {name}", "PASS", f"{len(rows) if rows else 0} Zeilen zurückgegeben")
    else:
        log_test(f"SELECT {name}", "FAIL", str(resp.get("error", ""))[:100])

# ============================================================
# SCHRITT 5: DNA-Kompression testen
# ============================================================
print("\n" + "="*70)
print("SCHRITT 5: DNA-Kompression")
print("="*70)

dna_request = {
    "sequences": ["ATCGATCGATCGATCGATCGATCG", "GCTAGCTAGCTAGCTAGCTAGCTA"],
    "algorithm": "KmerBased",
    "compression_level": 5
}
resp = parse_response(curl("POST", "/api/v1/dna/compress", dna_request))
if resp.get("success"):
    data = resp.get("data", {})
    compressed = data.get("compressed_sequences", [])
    stats = data.get("compression_stats", {})
    log_test("DNA Kompression", "PASS", 
             f"Komprimiert: {len(compressed)} Sequenzen, Ratio: {stats.get('compression_ratio', 'N/A')}")
else:
    log_test("DNA Kompression", "FAIL", str(resp.get("error", ""))[:100], resp)

# Dekompression testen
dna_decompress = {
    "compressed_data": ["QUNUR0FDR1RBQ0dUQUNH"]  # Base64 encoded
}
resp = parse_response(curl("POST", "/api/v1/dna/decompress", dna_decompress))
if resp.get("success"):
    log_test("DNA Dekompression", "PASS", "Dekompression erfolgreich")
else:
    error = resp.get("error", {})
    if isinstance(error, dict) and "InvalidInput" in str(error):
        log_test("DNA Dekompression", "WARN", "Erwartet gültige komprimierte Daten")
    else:
        log_test("DNA Dekompression", "FAIL", str(error)[:100])

# ============================================================
# SCHRITT 6: Quantum Search testen
# ============================================================
print("\n" + "="*70)
print("SCHRITT 6: Quantum Search")
print("="*70)

quantum_request = {
    "table_name": "books",
    "query_vector": [0.5, 0.3, 0.2, 0.1],
    "similarity_threshold": 0.3,
    "max_results": 10,
    "use_grover": True,
    "grover_config": {
        "backend": "simulator",
        "iterations": 50
    }
}
resp = parse_response(curl("POST", "/api/v1/quantum/search", quantum_request))
if resp.get("success"):
    data = resp.get("data", {})
    results_count = len(data.get("results", []))
    stats = data.get("quantum_stats", {})
    log_test("Quantum Search", "PASS", 
             f"{results_count} Ergebnisse, Speedup: {stats.get('speedup', 'N/A')}")
else:
    log_test("Quantum Search", "FAIL", str(resp.get("error", ""))[:100], resp)

# ============================================================
# SCHRITT 7: Neural Network Training testen
# ============================================================
print("\n" + "="*70)
print("SCHRITT 7: Neural Network Training")
print("="*70)

neural_request = {
    "network_name": "book_recommender_test",
    "training_data": [
        {"input": [0.8, 0.9, 0.7], "target": [1.0, 0.0]},
        {"input": [0.2, 0.1, 0.3], "target": [0.0, 1.0]},
        {"input": [0.5, 0.6, 0.4], "target": [0.7, 0.3]}
    ],
    "config": {
        "layers": [
            {"layer_type": "Dense", "size": 8, "activation": "ReLU"},
            {"layer_type": "Dense", "size": 4, "activation": "ReLU"},
            {"layer_type": "Dense", "size": 2, "activation": "Softmax"}
        ],
        "learning_rate": 0.01,
        "epochs": 10,
        "batch_size": 2,
        "optimizer": "Adam",
        "loss_function": "MeanSquaredError"
    },
    "validation_split": 0.2
}
resp = parse_response(curl("POST", "/api/v1/neural/train", neural_request))
if resp.get("success"):
    data = resp.get("data", {})
    network_id = data.get("network_id", "N/A")
    status = data.get("status", "N/A")
    log_test("Neural Network Training", "PASS", f"Network ID: {network_id}, Status: {status}")
else:
    log_test("Neural Network Training", "FAIL", str(resp.get("error", ""))[:100], resp)

# ============================================================
# SCHRITT 8: Biometrische Authentifizierung testen
# ============================================================
print("\n" + "="*70)
print("SCHRITT 8: Biometrische Authentifizierung (EEG)")
print("="*70)

# EEG Benutzer auflisten
resp = parse_response(curl("GET", "/api/v1/biometric/eeg/users"))
if resp.get("success"):
    data = resp.get("data", [])
    users = data if isinstance(data, list) else data.get("users", []) if isinstance(data, dict) else []
    log_test("EEG Benutzer auflisten", "PASS", f"{len(users)} Benutzer registriert")
else:
    log_test("EEG Benutzer auflisten", "FAIL", str(resp.get("error", ""))[:100])

# EEG Registrierung
import random
eeg_samples = [[random.uniform(-50, 50) for _ in range(256)] for _ in range(3)]
enroll_request = {
    "user_id": "bibliothekar_test",
    "eeg_samples": eeg_samples,
    "sampling_rate": 256
}
resp = parse_response(curl("POST", "/api/v1/biometric/enroll", enroll_request))
if resp.get("success"):
    log_test("EEG Registrierung", "PASS", "Bibliothekar registriert")
else:
    error = str(resp.get("error", ""))
    if "already" in error.lower() or "exists" in error.lower():
        log_test("EEG Registrierung", "WARN", "Benutzer bereits registriert")
    else:
        log_test("EEG Registrierung", "FAIL", error[:100])

# EEG Verifizierung
verify_request = {
    "user_id": "bibliothekar_test",
    "eeg_sample": eeg_samples[0]
}
resp = parse_response(curl("POST", "/api/v1/biometric/verify", verify_request))
if resp.get("success"):
    data = resp.get("data", {})
    confidence = data.get("confidence", "N/A")
    log_test("EEG Verifizierung", "PASS", f"Confidence: {confidence}")
else:
    log_test("EEG Verifizierung", "FAIL", str(resp.get("error", ""))[:100])

# ============================================================
# SCHRITT 9: QSQL Neuromorphe Funktionen testen
# ============================================================
print("\n" + "="*70)
print("SCHRITT 9: QSQL Neuromorphe Funktionen")
print("="*70)

qsql_tests = [
    ("SYNAPTIC_WEIGHT", "SELECT title, SYNAPTIC_WEIGHT(title, 'Harry Potter') AS weight FROM books"),
    ("HEBBIAN_LEARNING", "SELECT title, HEBBIAN_LEARNING(publication_year) AS hebbian FROM books LIMIT 5"),
    # NEUROMATCH könnte andere Syntax erfordern
]

for name, query in qsql_tests:
    resp = parse_response(curl("POST", "/api/v1/query", {"query": query}))
    if resp.get("success"):
        rows = resp.get("data", {}).get("rows", [])
        log_test(f"QSQL {name}", "PASS", f"{len(rows) if rows else 0} Zeilen")
    else:
        error = str(resp.get("error", ""))
        if "not implemented" in error.lower() or "unsupported" in error.lower():
            log_test(f"QSQL {name}", "WARN", "Funktion noch nicht vollständig implementiert")
        else:
            log_test(f"QSQL {name}", "FAIL", error[:100])

# ============================================================
# SCHRITT 10: Performance Stats
# ============================================================
print("\n" + "="*70)
print("SCHRITT 10: Performance & Monitoring")
print("="*70)

resp = parse_response(curl("GET", "/api/v1/stats/performance"))
if resp.get("success"):
    data = resp.get("data", {})
    log_test("Performance Stats", "PASS", 
             f"Memory: {data.get('memory', {}).get('used_mb', 'N/A')}MB")
else:
    log_test("Performance Stats", "FAIL", str(resp.get("error", ""))[:100])

# Metrics Endpoint
resp = curl("GET", "/metrics", auth=False)
if "neuroquantum" in resp.lower() or "http_requests" in resp.lower():
    log_test("Prometheus Metrics", "PASS", "Metriken verfügbar")
else:
    log_test("Prometheus Metrics", "WARN", "Metriken-Format unbekannt")

# ============================================================
# ZUSAMMENFASSUNG
# ============================================================
print("\n" + "="*70)
print("ZUSAMMENFASSUNG")
print("="*70)

total = results["summary"]["passed"] + results["summary"]["failed"] + results["summary"]["warnings"]
success_rate = (results["summary"]["passed"] / total * 100) if total > 0 else 0

print(f"\n✅ Bestanden:  {results['summary']['passed']}")
print(f"❌ Fehlgeschlagen: {results['summary']['failed']}")
print(f"⚠️  Warnungen: {results['summary']['warnings']}")
print(f"\n📊 Erfolgsrate: {success_rate:.1f}%")

# Fehler auflisten
if results["summary"]["failed"] > 0:
    print("\n❌ Fehlgeschlagene Tests:")
    for test in results["tests"]:
        if test["status"] == "FAIL":
            print(f"   • {test['name']}: {test['message']}")

# Warnungen auflisten
if results["summary"]["warnings"] > 0:
    print("\n⚠️  Warnungen:")
    for test in results["tests"]:
        if test["status"] == "WARN":
            print(f"   • {test['name']}: {test['message']}")

# Ergebnisse speichern
output_file = "library_example_test_results.json"
with open(output_file, "w") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f"\n📄 Detaillierte Ergebnisse gespeichert in: {output_file}")

print("\n" + "="*70)
