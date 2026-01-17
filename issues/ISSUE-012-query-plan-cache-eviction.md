# ISSUE-012: Query Plan Cache Eviction

**Priorität:** 🟢 NIEDRIG  
**Aufwand:** 4-6 Stunden  
**Status:** ⬜ Offen  
**Sprint:** 6 (Nice-to-Have)

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

- [ ] Memory-Limit konfigurierbar
- [ ] LRU-Eviction implementiert
- [ ] Cache wächst nicht unbegrenzt
