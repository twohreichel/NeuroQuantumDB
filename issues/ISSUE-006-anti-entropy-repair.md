# ISSUE-006: Anti-Entropy Repair implementieren

**Priorität:** 🟠 HOCH  
**Aufwand:** 16-24 Stunden  
**Status:** ⬜ Offen  
**Sprint:** 5 (Cluster-Stabilität)

---

## Problembeschreibung

Die Shard-Reparatur-Funktion gibt nur Nullwerte zurück, ohne tatsächlich Reparaturen durchzuführen.

## Betroffene Dateien

- `crates/neuroquantum-cluster/src/replication.rs` (Zeile ~366-387)

## Aktueller Code

```rust
pub async fn repair_shard(...) -> ClusterResult<RepairResult> {
    // In a full implementation:
    // 1. Exchange Merkle tree roots with replicas
    // 2. Identify divergent ranges
    // 3. Stream differing data
    // 4. Apply repairs

    Ok(RepairResult {
        shard_id,
        keys_repaired: 0,  // Immer 0!
        bytes_transferred: 0,
        duration_ms: 0,
    })
}
```

## Impact

- Daten-Inkonsistenzen zwischen Replikas werden nicht automatisch korrigiert
- Cluster-Integrität nicht gewährleistet
- Manuelles Eingreifen bei Daten-Divergenz nötig

---

## Lösungsschritte

### Schritt 1: Funktion finden
```bash
grep -n "repair_shard\|RepairResult\|merkle" crates/neuroquantum-cluster/src/replication.rs
```

### Schritt 2: Implementation
1. Implementiere Merkle-Tree-Vergleich zwischen Replikas
2. Identifiziere divergente Key-Bereiche
3. Implementiere Data-Streaming für differierende Daten
4. Implementiere Conflict Resolution Strategy (Last-Write-Wins, Vector Clocks)

---

## Validierung

```bash
cargo test -p neuroquantum-cluster repair -- --nocapture
```

## Akzeptanzkriterium

- [ ] Merkle-Tree-Vergleich implementiert
- [ ] Divergente Keys werden identifiziert
- [ ] Automatische Reparatur funktioniert
- [ ] `keys_repaired` und `bytes_transferred` korrekt
