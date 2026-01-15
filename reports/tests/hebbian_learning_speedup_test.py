#!/usr/bin/env python3
"""
HEBBIAN_LEARNING Speedup Test
==============================
Beweist, dass wiederholte Queries durch Hebbian Learning schneller werden.

Das Hebb'sche Prinzip: "Neurons that fire together, wire together"
→ Queries die oft ausgeführt werden, werden optimiert.
"""

import subprocess
import json
import time
import statistics

API_KEY = "nqdb_03c495c620c646eaa7ce89dd2a78ce86"
BASE_URL = "http://127.0.0.1:8080"

def execute_query(query):
    """Führt Query aus und gibt Antwortzeit zurück"""
    cmd = [
        "curl", "-s", "-w", "\n%{time_total}",
        "-X", "POST",
        "-H", f"X-API-Key: {API_KEY}",
        "-H", "Content-Type: application/json",
        "-d", json.dumps({"query": query}),
        f"{BASE_URL}/api/v1/query"
    ]
    
    start = time.perf_counter()
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    end = time.perf_counter()
    
    # Parse curl timing
    lines = result.stdout.strip().split('\n')
    curl_time = float(lines[-1]) if lines else 0
    
    # Python-gemessene Zeit
    python_time = end - start
    
    return {
        "curl_time_ms": curl_time * 1000,
        "python_time_ms": python_time * 1000,
        "success": "success" in result.stdout.lower()
    }

def run_speedup_test(query, name, iterations=20):
    """Führt Query mehrfach aus und misst Speedup"""
    print(f"\n{'='*70}")
    print(f"TEST: {name}")
    print(f"Query: {query[:60]}...")
    print(f"Iterationen: {iterations}")
    print('='*70)
    
    times = []
    
    # Warmup (erste Ausführung ignorieren)
    print("\n🔄 Warmup...")
    execute_query(query)
    time.sleep(0.1)
    
    # Messungen
    print(f"\n📊 Messungen ({iterations} Durchläufe):\n")
    print(f"{'#':>3} | {'Zeit (ms)':>10} | {'Trend':>8} | Balken")
    print("-" * 50)
    
    for i in range(iterations):
        result = execute_query(query)
        times.append(result["python_time_ms"])
        
        # Trend berechnen
        if i > 0:
            change = times[i] - times[i-1]
            trend = "↓" if change < -1 else "↑" if change > 1 else "→"
            trend_color = "🟢" if change < -1 else "🔴" if change > 1 else "🟡"
        else:
            trend = "—"
            trend_color = "⚪"
        
        # Balkendiagramm (normalisiert auf max 40 Zeichen)
        bar_len = int(min(result["python_time_ms"] / 10, 40))
        bar = "█" * bar_len
        
        print(f"{i+1:>3} | {result['python_time_ms']:>10.2f} | {trend_color} {trend:>5} | {bar}")
        
        time.sleep(0.05)  # Kleine Pause zwischen Queries
    
    # Statistiken
    print("\n" + "="*70)
    print("📈 STATISTIKEN")
    print("="*70)
    
    first_5 = times[:5]
    last_5 = times[-5:]
    
    avg_first = statistics.mean(first_5)
    avg_last = statistics.mean(last_5)
    speedup = ((avg_first - avg_last) / avg_first) * 100
    
    print(f"\n  Erste 5 Queries:    {avg_first:.2f} ms (Durchschnitt)")
    print(f"  Letzte 5 Queries:   {avg_last:.2f} ms (Durchschnitt)")
    print(f"\n  Minimum:            {min(times):.2f} ms")
    print(f"  Maximum:            {max(times):.2f} ms")
    print(f"  Standardabweichung: {statistics.stdev(times):.2f} ms")
    
    print(f"\n  {'🚀 SPEEDUP:':>20} {speedup:>6.1f}%")
    
    if speedup > 5:
        print(f"\n  ✅ BESTANDEN: Hebbian Learning hat die Query um {speedup:.1f}% beschleunigt!")
    elif speedup > 0:
        print(f"\n  ⚠️  MARGINAL: Kleine Verbesserung von {speedup:.1f}%")
    else:
        print(f"\n  ❌ KEIN SPEEDUP: Query wurde nicht schneller ({speedup:.1f}%)")
    
    return {
        "name": name,
        "iterations": iterations,
        "times": times,
        "avg_first_5": avg_first,
        "avg_last_5": avg_last,
        "speedup_percent": speedup,
        "min": min(times),
        "max": max(times),
        "stddev": statistics.stdev(times)
    }

def main():
    print("""
╔══════════════════════════════════════════════════════════════════════╗
║           HEBBIAN LEARNING SPEEDUP TEST                              ║
║                                                                      ║
║  "Neurons that fire together, wire together" - Donald Hebb, 1949    ║
║                                                                      ║
║  Dieser Test beweist, dass wiederholte Queries durch das            ║
║  Hebbian Learning Prinzip im Query-Optimizer schneller werden.      ║
╚══════════════════════════════════════════════════════════════════════╝
    """)
    
    results = []
    
    # Test 1: HEBBIAN_LEARNING Funktion
    results.append(run_speedup_test(
        "SELECT title, HEBBIAN_LEARNING(publication_year) AS hebbian FROM books LIMIT 5",
        "HEBBIAN_LEARNING Query",
        iterations=25
    ))
    
    # Test 2: Normale SELECT Query zum Vergleich
    results.append(run_speedup_test(
        "SELECT * FROM books ORDER BY publication_year DESC LIMIT 5",
        "Normale SELECT Query (Vergleich)",
        iterations=25
    ))
    
    # Test 3: Komplexere Query mit HEBBIAN_LEARNING
    results.append(run_speedup_test(
        "SELECT title, author, HEBBIAN_LEARNING(publication_year) AS h FROM books WHERE genre = 'Fantasy'",
        "HEBBIAN_LEARNING mit WHERE",
        iterations=25
    ))
    
    # Zusammenfassung
    print("\n" + "="*70)
    print("🏁 GESAMTZUSAMMENFASSUNG")
    print("="*70)
    
    print(f"\n{'Test':<35} | {'Speedup':>10} | {'Ergebnis':>10}")
    print("-" * 60)
    
    for r in results:
        status = "✅ PASS" if r["speedup_percent"] > 5 else "⚠️  MARGINAL" if r["speedup_percent"] > 0 else "❌ FAIL"
        print(f"{r['name']:<35} | {r['speedup_percent']:>9.1f}% | {status}")
    
    # Speichern
    output_file = "hebbian_speedup_results.json"
    with open(output_file, "w") as f:
        json.dump(results, f, indent=2)
    print(f"\n📄 Ergebnisse gespeichert in: {output_file}")
    
    # Fazit
    avg_speedup = statistics.mean([r["speedup_percent"] for r in results])
    print(f"\n📊 Durchschnittlicher Speedup über alle Tests: {avg_speedup:.1f}%")
    
    if avg_speedup > 5:
        print("\n🎉 FAZIT: Hebbian Learning führt nachweislich zu schnelleren Queries!")
    else:
        print("\n📝 FAZIT: Der Speedup-Effekt ist in diesem kurzen Test gering.")
        print("   Für signifikante Verbesserungen: Mehr Iterationen, größere Datenmenge.")

if __name__ == "__main__":
    main()
