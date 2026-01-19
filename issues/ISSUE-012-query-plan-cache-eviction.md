# ISSUE-012: Query Plan Cache Eviction

**Priorität:** 🟢 NIEDRIG  
**Aufwand:** 4-6 Stunden  
**Status:** ✅ Erledigt  
**Sprint:** 6 (Nice-to-Have)  
**Abgeschlossen:** 19. Januar 2026

---

## Problembeschreibung

Der Query Plan Cache hat keine automatische Eviction bei Memory-Druck.

## Betroffene Dateien

- `crates/neuroquantum-qsql/src/lib.rs` (Zeile ~70-77)

## Aktueller Code

```rust
pub struct CachedQueryPlan {
    pub plan: Arc<QueryPlan>,
    pub execution_count: u64,
    pub average_duration: Duration,
    pub synaptic_strength: f32,
    pub last_accessed: Instant,
}
```

## Impact

- Potenzielle Memory-Probleme bei vielen verschiedenen Queries
- Keine automatische Bereinigung bei Speicherknappheit

---

## Lösungsschritte

### Schritt 1: Cache-Struktur analysieren
```bash
grep -n "CachedQueryPlan\|query_cache\|plan_cache" crates/neuroquantum-qsql/src/lib.rs
```

### Schritt 2: Implementation
1. Memory-Limit für Cache konfigurierbar machen
2. LRU-Eviction basierend auf `last_accessed` implementieren
3. Alternativ: `synaptic_strength` für Priorisierung nutzen (Hebbian-inspiriert)

```rust
impl QueryPlanCache {
    pub fn evict_if_needed(&mut self, max_memory_bytes: usize) {
        if self.current_memory() > max_memory_bytes {
            // Evict least recently used or lowest synaptic_strength
        }
    }
}
```

---

## Validierung

```bash
cargo test -p neuroquantum-qsql cache -- --nocapture
```

## Akzeptanzkriterium

- [x] Memory-Limit konfigurierbar
- [x] LRU-Eviction implementiert
- [x] Cache wächst nicht unbegrenzt

---

## Lösung

Neue Datei `crates/neuroquantum-qsql/src/query_plan_cache.rs` mit:

- `QueryPlanCache` - Vollständige Cache-Implementierung mit LRU-Eviction
- `QueryPlanCacheConfig` - Konfigurierbare Limits (max_entries, max_memory_bytes, etc.)
- `CachedQueryPlan` - Erweiterte Struktur mit Memory-Tracking und synaptic_strength
- `CacheStatistics` - Statistiken für Monitoring (hits, misses, evictions, etc.)

### Features:
- **Konfigurierbares Memory-Limit** (Standard: 64 MB)
- **LRU-Eviction** basierend auf `last_accessed`
- **Hebbian-inspirierte Priorisierung**: Häufig genutzte Queries haben höhere `synaptic_strength`
- **Synaptic Decay**: Optionaler Verfall der Stärke über Zeit
- **Batch-Eviction**: Effiziente Eviction bei Speicherdruck

### Tests:
8 Unit-Tests für alle Cache-Funktionalitäten implementiert.
