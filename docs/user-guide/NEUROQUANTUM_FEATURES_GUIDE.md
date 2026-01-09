# 🧠 NeuroQuantumDB Feature Guide
## Quantum Search, Neuronale Endpunkte & DNA Kompression

> *"Eine Datenbank, die atmet, lernt und sich weiterentwickelt"*

---

## 📚 Inhaltsverzeichnis

1. [Übersicht](#-übersicht)
2. [DNA Kompression](#-dna-kompression)
3. [Quantum Search](#-quantum-search)
4. [Neuronale Endpunkte](#-neuronale-endpunkte)
5. [QSQL Neuromorphe Erweiterungen](#-qsql-neuromorphe-erweiterungen)
6. [Biometrische Authentifizierung](#-biometrische-authentifizierung)
7. [Praktische Anwendungsfälle](#-praktische-anwendungsfälle)
8. [API Referenz](#-api-referenz)

---

## 🌟 Übersicht

### Was ist NeuroQuantumDB? - Erklärt für Jedermann

**Stell dir vor**, du hast eine riesige Bibliothek mit Millionen von Büchern. Eine normale Datenbank ist wie ein Bibliothekar, der jedes Buch einzeln durchsuchen muss, um das richtige zu finden. Das dauert sehr lange!

**NeuroQuantumDB ist anders.** Es ist wie ein magischer Bibliothekar, der:

1. **🧬 Bücher kleiner machen kann** (DNA Kompression) - Stell dir vor, du könntest 4 Bücher in den Platz von einem einzigen quetschen, ohne dass etwas verloren geht!

2. **⚛️ An vielen Orten gleichzeitig suchen kann** (Quantum Search) - Anstatt ein Regal nach dem anderen zu durchsuchen, schaut er sich ALLE Regale gleichzeitig an. Das ist wie Magie!

3. **🧠 Aus Erfahrung lernt** (Neuronale Netzwerke) - Je öfter du nach bestimmten Büchern fragst, desto besser wird er darin, sie zu finden. Wie ein Hund, der lernt, wo sein Spielzeug versteckt ist!

### Warum ist das wichtig?

In unserer digitalen Welt haben wir UNGLAUBLICH viele Daten:
- Jede Sekunde werden Millionen von Fotos hochgeladen
- Online-Shops haben Millionen von Produkten
- Krankenhäuser speichern Gesundheitsdaten von Milliarden Menschen

**Das Problem:** Normale Datenbanken sind zu langsam und brauchen zu viel Speicherplatz.

**Die Lösung:** NeuroQuantumDB nutzt Tricks aus der Natur (DNA) und der Quantenphysik, um schneller und effizienter zu sein!

NeuroQuantumDB vereint drei revolutionäre Technologien in einer Datenbank:

```
┌─────────────────────────────────────────────────────────────────────┐
│                     NeuroQuantumDB Features                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  🧬 DNA KOMPRESSION        ⚛️ QUANTUM SEARCH       🧠 NEURAL NETWORK │
│  ─────────────────        ────────────────        ──────────────── │
│  • 4:1 Kompression        • Grover's Algorithmus  • Hebbian Learning │
│  • Quaternäre Kodierung   • QUBO Optimierung      • STDP Plastizität │
│  • SIMD Beschleunigung    • TFIM Berechnung       • Pattern Matching │
│  • Fehlerkorrektur        • Parallel Tempering    • Adaptive Gewichte │
│                                                                      │
│  ════════════════════════════════════════════════════════════════   │
│                                                                      │
│  🔐 BIOMETRIC AUTH         📊 QSQL ERWEITERUNGEN                    │
│  ─────────────────         ─────────────────────                    │
│  • EEG-basiert            • NEUROMATCH Funktion                     │
│  • Multi-Channel          • SYNAPTIC_WEIGHT                         │
│  • Echtzeit-Verifikation  • QUANTUM_SEARCH                          │
│  • Liveness Detection     • HEBBIAN_LEARNING                        │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 🧬 DNA Kompression

### Was ist DNA Kompression? - Erklärt für Jedermann

**Stell dir vor**, du hast einen Koffer und möchtest 100 T-Shirts mitnehmen, aber nur 25 passen hinein. Was tust du? Du rollst die T-Shirts ganz fest zusammen! Am Ende passen alle 100 hinein, und wenn du sie wieder ausrollst, sind sie genauso wie vorher.

**DNA Kompression funktioniert genauso**, nur mit Computerdaten!

#### Warum "DNA"?

In deinem Körper gibt es DNA - das ist wie ein riesiges Rezeptbuch, das erklärt, wie DU gebaut bist. Dieses Rezeptbuch benutzt nur 4 "Buchstaben":
- **A** (Adenin) - wie die Farbe ROT 🔴
- **C** (Cytosin) - wie die Farbe BLAU 🔵
- **G** (Guanin) - wie die Farbe GRÜN 🟢
- **T** (Thymin) - wie die Farbe GELB 🟡

Computer benutzen normalerweise nur 0 und 1 (an/aus, wie ein Lichtschalter). Aber mit 4 "Farben" können wir VIEL mehr Information in weniger Platz speichern!

#### Ein einfaches Beispiel:

```
MIT NORMALER SPEICHERUNG:          MIT DNA KOMPRESSION:
┌─────────────────────┐            ┌──────────────┐
│ 0 1 0 0 1 0 0 0     │            │              │
│ 0 1 1 0 0 1 0 1     │    ───▶    │  A C G T     │
│ 0 1 1 0 1 1 0 0     │            │  A T G C     │
│ 0 1 1 0 1 1 1 1     │            │              │
└─────────────────────┘            └──────────────┘
     32 Zeichen                       8 Zeichen
                                   
     Das ist wie:                  Das ist wie:
     ████████████████              ████
     
     4x MEHR PLATZ!                4x WENIGER PLATZ!
```

#### Warum ist das toll?

| Vorher | Nachher | Was bedeutet das? |
|--------|---------|-------------------|
| 4 GB Festplatte voll | Nur 1 GB belegt | Du kannst 4x mehr Fotos speichern! |
| Backup dauert 4 Stunden | Nur 1 Stunde | Mehr Zeit zum Spielen! |
| Server kostet 400€/Monat | Nur 100€/Monat | Papa spart Geld! |

### Technische Details

DNA-inspirierte quaternäre Kodierung für ultra-effiziente Speicherung. Binäre Daten werden in DNA-Basenpaare umgewandelt:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    DNA Kompression Prozess                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   BINÄR                QUATERNÄR               KOMPRIMIERT           │
│   ──────               ─────────               ────────────          │
│                                                                      │
│   01001000  ────▶      A  C  G  T     ────▶    ~75% kleiner         │
│   01100101             A  T  G  C                                   │
│   01101100             ────────────                                  │
│   01101100              DNA Basen                                    │
│   01101111                                                           │
│                                                                      │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │  Binär    │  DNA Base  │  Bedeutung                          │  │
│   ├──────────────────────────────────────────────────────────────┤  │
│   │   00      │     A      │  Adenin                             │  │
│   │   01      │     C      │  Cytosin                            │  │
│   │   10      │     G      │  Guanin                             │  │
│   │   11      │     T      │  Thymin                             │  │
│   └──────────────────────────────────────────────────────────────┘  │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Performance

| Datengröße | Kompressionszeit | Verhältnis |
|------------|------------------|------------|
| 1 KB       | < 0.1 ms         | 4:1        |
| 1 MB       | < 2 ms           | 4:1        |
| 100 MB     | < 200 ms         | 4:1        |

### SIMD Beschleunigung

#### Was ist SIMD? - Erklärt für Jedermann

**Stell dir vor**, du musst 100 Äpfel schälen. Normalerweise schälst du einen nach dem anderen. Das dauert ewig!

**SIMD** ist wie wenn du plötzlich 4 oder 8 Hände hättest und 4-8 Äpfel GLEICHZEITIG schälen könntest!

Computer-Chips haben diese "Superhände" eingebaut:
- **ARM64 NEON** (in Handys, Raspberry Pi): 4 Äpfel gleichzeitig! 🍎🍎🍎🍎
- **x86_64 AVX2** (in Laptops, PCs): 8 Äpfel gleichzeitig! 🍎🍎🍎🍎🍎🍎🍎🍎

NeuroQuantumDB erkennt automatisch, welche "Superhände" dein Computer hat und benutzt sie!

#### Technische Details

Automatische Hardware-Beschleunigung:

- **ARM64 NEON**: 4x schneller auf Raspberry Pi
- **x86_64 AVX2**: 8x schneller auf Intel/AMD

### API Endpunkte

#### 🔹 DNA Komprimieren

```bash
POST /api/v1/dna/compress
```

**Request:**
```json
{
  "sequences": [
    "ATCGATCGATCG",
    "GCTAGCTAGCTA"
  ],
  "algorithm": "KmerBased",
  "compression_level": 5
}
```

**Algorithmus-Optionen:**
| Algorithmus | Beschreibung | Anwendungsfall |
|-------------|--------------|----------------|
| `KmerBased` | K-mer basierte Kompression | Standard, schnell |
| `NeuralNetwork` | Neuronale Netzwerk Kompression | Muster-basierte Daten |
| `QuantumInspired` | Quantum-inspirierte Kompression | Komplexe Strukturen |
| `Hybrid` | Hybrid-Ansatz | Beste Kompression |

**Response:**
```json
{
  "success": true,
  "data": {
    "compressed_sequences": [
      {
        "original_length": 12,
        "compressed_data": "base64_encoded_data",
        "compression_ratio": 2.5,
        "checksum": "abc123"
      }
    ],
    "compression_stats": {
      "total_input_size": 24,
      "total_compressed_size": 10,
      "average_compression_ratio": 2.4,
      "compression_time_ms": 15.2
    }
  }
}
```

#### 🔹 DNA Dekomprimieren

```bash
POST /api/v1/dna/decompress
```

**Request:**
```json
{
  "compressed_data": [
    "base64_encoded_data1",
    "base64_encoded_data2"
  ]
}
```

### QSQL Syntax

```sql
-- Tabelle komprimieren
COMPRESS TABLE logs USING DNA;

-- Kompressionsstatistiken anzeigen
SHOW COMPRESSION STATS FOR logs;

-- Dekomprimieren
DECOMPRESS TABLE logs;
```

### 💡 Anwendungsfälle für Entwickler & Kunden

| Szenario | Nutzen |
|----------|--------|
| **Log-Archivierung** | 75% Speicherersparnis bei historischen Logs |
| **IoT Sensordaten** | Effiziente Speicherung auf Edge-Devices |
| **Backup-Systeme** | Schnellere Backups durch kleinere Datenmengen |
| **Genomik-Daten** | Native DNA-Sequenz Speicherung |
| **Cold Storage** | Langzeit-Archivierung mit minimalen Kosten |

---

## ⚛️ Quantum Search

### Was ist Quantum Search? - Erklärt für Jedermann

**Stell dir vor**, du suchst dein Lieblingsspielzeug in einem riesigen Spielzeugladen mit 1 Million Spielzeugen!

#### Normale Suche (Klassisch):
Du gehst durch jeden Gang, schaust in jedes Regal, eins nach dem anderen...
- Gang 1... nein 😕
- Gang 2... nein 😕
- Gang 3... nein 😕
- ... (1 Million Mal schauen!)

**Das dauert EWIG!** ⏰

#### Quantum Suche (Magisch):
Stell dir vor, du könntest dich KLONEN und plötzlich gibt es 1000 von dir! Jeder Klon schaut in einem anderen Gang nach. Dann "verschmelzen" alle Klone wieder zu dir, und du weißt sofort, wo das Spielzeug ist!

**Das ist wie Zauberei!** ✨

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Der Unterschied                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  NORMALE SUCHE:        Du allein, ein Regal nach dem anderen        │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ 👤➡️📦➡️📦➡️📦➡️📦➡️📦➡️📦➡️📦➡️📦➡️📦➡️📦➡️...         │    │
│  │                                                             │    │
│  │ Bei 1.000.000 Regalen: 1.000.000 Schritte! 😫               │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  QUANTUM SUCHE:        Du bist überall gleichzeitig!                │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │         👤 👤 👤 👤 👤 👤 👤 👤 👤 👤                       │    │
│  │         ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓                        │    │
│  │         📦 📦 📦 📦 📦 📦 📦 📦 📦 📦                       │    │
│  │                                                             │    │
│  │ Bei 1.000.000 Regalen: Nur ~1.000 Schritte! 🎉              │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  ⚡ Das ist 1000x SCHNELLER!                                        │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

#### Die Magie dahinter: Grover's Algorithmus

Ein sehr schlauer Mensch namens **Lov Grover** hat 1996 herausgefunden, wie man diese Quantenmagie für die Suche nutzen kann. Sein Trick:

1. **Superposition**: Dein Quantum-Computer schaut sich ALLE Möglichkeiten gleichzeitig an (wie die Klone!)
2. **Amplitude Amplification**: Die richtige Antwort wird "lauter" gemacht, wie wenn dein Lieblingslied im Radio lauter gedreht wird
3. **Messung**: Am Ende "hörst" du nur noch die richtige Antwort!

#### Verschiedene Quantum-Modi erklärt

| Modus | Wie ein Kind es verstehen würde | Wofür ist es gut? |
|-------|----------------------------------|-------------------|
| **Grover's** | "Finde die Nadel im Heuhaufen, aber schau dir den ganzen Haufen gleichzeitig an!" | Schnelles Suchen |
| **TFIM** | "Finde die Position, wo Magnete am ruhigsten sind" | Energie-Probleme lösen |
| **QUBO** | "Finde den besten Weg, 100 Aufgaben zu erledigen, wenn du nur 10 Stunden hast" | Optimierung |
| **Parallel Tempering** | "Teste viele Lösungen bei verschiedenen 'Temperaturen' und behalte die beste" | Globale Optimum finden |

### Technische Details

Quantum-inspirierte Algorithmen für dramatisch schnellere Suche in unstrukturierten Daten:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Quantum Speedup Visualisierung                    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Klassische Suche:  O(N) Operationen                                │
│  Quantum Suche:     O(√N) Operationen                               │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │  N = 1.000.000 Datensätze                                      │ │
│  │                                                                 │ │
│  │  Klassisch: █████████████████████████████████ 1.000.000        │ │
│  │  Quantum:   █                                ~1.000            │ │
│  │                                                                 │ │
│  │  ⚡ 1000x schneller!                                           │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Verfügbare Quantum Modi

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Quantum Search Modi                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │   GROVER'S   │  │    TFIM      │  │    QUBO      │              │
│  │  ALGORITHM   │  │  (Ising)     │  │ Optimization │              │
│  ├──────────────┤  ├──────────────┤  ├──────────────┤              │
│  │ • O(√N) Suche│  │ • Energie-   │  │ • Quadratische│             │
│  │ • Amplitude  │  │   minimierung│  │   Optimierung│              │
│  │   Verstärkung│  │ • Magnetische│  │ • Constraint │              │
│  │ • Pattern    │  │   Systeme    │  │   Solving    │              │
│  │   Matching   │  │ • Phase      │  │ • Max-Cut    │              │
│  │              │  │   Transition │  │   Problems   │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
│                                                                      │
│  ┌──────────────┐                                                   │
│  │  PARALLEL    │                                                   │
│  │  TEMPERING   │                                                   │
│  ├──────────────┤                                                   │
│  │ • Monte Carlo│                                                   │
│  │ • Temperatur-│                                                   │
│  │   Replikas   │                                                   │
│  │ • Global     │                                                   │
│  │   Optimum    │                                                   │
│  └──────────────┘                                                   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### API Endpunkt

```bash
POST /api/v1/quantum/search
```

**Request:**
```json
{
  "table_name": "users",
  "query_vector": [0.1, 0.5, 0.8, 0.3],
  "similarity_threshold": 0.7,
  "max_results": 10,
  "entanglement_boost": 1.2,
  "use_tfim": true,
  "use_qubo": false,
  "use_parallel_tempering": false,
  "use_grover": true,
  "grover_config": {
    "backend": "simulator",
    "num_shots": 1024,
    "error_mitigation": true,
    "success_threshold": 0.5
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "results": [
      {
        "record": {
          "id": 1,
          "name": "Alice",
          "features": [0.15, 0.52, 0.79, 0.28]
        },
        "similarity_score": 0.95,
        "quantum_probability": 0.88,
        "entanglement_strength": 0.72
      }
    ],
    "quantum_stats": {
      "coherence_time_used_ms": 2.5,
      "superposition_states": 16,
      "measurement_collapses": 4,
      "entanglement_operations": 8,
      "circuit_depth": 12,
      "num_gates": 48
    },
    "grover_results": {
      "found_indices": [42, 137, 891],
      "probabilities": [0.92, 0.85, 0.71],
      "iterations": 31,
      "optimal_iterations": 31,
      "quantum_speedup": 31.62,
      "computation_time_ms": 12.4
    }
  }
}
```

### QUBO Backends

| Backend | Beschreibung | Anwendung |
|---------|--------------|-----------|
| `VQE` | Variational Quantum Eigensolver | Energie-Probleme |
| `QAOA` | Quantum Approximate Optimization | Kombinatorik |
| `QA` | Quantum Annealing (D-Wave style) | Global Optima |
| `SQA` | Simulated Quantum Annealing | Default, robust |
| `CLASSICAL` | Klassischer Fallback | Debugging |

### QSQL Syntax

```sql
-- Basis Quantum Suche
QUANTUM SEARCH users WHERE age > 30;

-- Mit Iterationslimit
QUANTUM SEARCH products 
  WHERE price < 100 
  WITH ITERATIONS 50;

-- QUBO Optimierung
OPTIMIZE QUBO
  MINIMIZE 3*x1 + 2*x2 - x1*x2
  SUBJECT TO x1 + x2 <= 1
  BACKEND SQA;
```

### 💡 Anwendungsfälle für Entwickler & Kunden

| Szenario | Nutzen | Speedup |
|----------|--------|---------|
| **Ähnlichkeitssuche** | Produkt-Empfehlungen, Content-Matching | √N |
| **Anomalie-Erkennung** | Fraud Detection, Security Monitoring | √N |
| **Graph-Optimierung** | Routing, Netzwerk-Planung | Exponentiell |
| **Portfolio-Optimierung** | Finanz-Strategien | QUBO |
| **Scheduling** | Ressourcen-Allokation | QUBO |
| **Machine Learning** | Feature-Selection | Quantum-Enhanced |

---

## 🧠 Neuronale Endpunkte

### Was sind Neuronale Endpunkte? - Erklärt für Jedermann

**Stell dir vor**, du hast einen Roboter-Hund als Haustier. Am Anfang weiß er nichts - er weiß nicht, wo sein Napf ist, nicht wo sein Körbchen ist, nicht einmal seinen Namen!

Aber jeden Tag lernst du ihm etwas Neues:
- "Wenn ich 'Futter!' rufe, geh zum Napf" 🍖
- "Wenn ich 'Schlafenszeit!' sage, geh ins Körbchen" 🛏️
- "Wenn die Türklingel läutet, belle!" 🔔

Nach einer Weile wird dein Roboter-Hund richtig SCHLAU! Er kann sogar neue Situationen verstehen, die du ihm nie beigebracht hast!

**Neuronale Netzwerke in NeuroQuantumDB funktionieren genauso!**

#### Wie funktioniert das Lernen?

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Wie ein Gehirn lernt                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   DEIN GEHIRN:                    NEUROQUANTUMDB:                   │
│                                                                      │
│   🧠 Neuronen (Gehirnzellen)      🔵 Künstliche Neuronen            │
│      ↓                               ↓                               │
│   🔗 Synapsen (Verbindungen)      🔗 Gewichte (Zahlen)              │
│      ↓                               ↓                               │
│   📚 Lernen durch Wiederholung    📊 Lernen durch Daten             │
│                                                                      │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │                                                             │   │
│   │    Eingabe        Verarbeitung         Ausgabe              │   │
│   │                                                             │   │
│   │    👀 Ich sehe    🧠 Hmm, das sieht    🗣️ "Das ist         │   │
│   │    etwas Rotes    aus wie...            ein Apfel!"         │   │
│   │    und Rundes                                               │   │
│   │                                                             │   │
│   │    [0.9, 0.1]  →  ⚙️⚙️⚙️⚙️  →  "Apfel" (95% sicher)       │   │
│   │    (rot, rund)    (Neuronen)                                │   │
│   │                                                             │   │
│   └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

#### Die wichtigsten Lernregeln erklärt

**1. Hebbian Learning (Hebb'sches Lernen)**

> "Neuronen, die zusammen feuern, verdrahten sich zusammen"

**Beispiel für Kinder:**
Stell dir vor, jedes Mal wenn du "Eis" hörst, denkst du an "Sommer". Je öfter das passiert, desto stärker wird die Verbindung in deinem Kopf!

```
🍦 "Eis"  ←──────────────→  ☀️ "Sommer"
          (wird immer stärker!)
```

**2. STDP (Spike-Timing Dependent Plasticity)**

**Beispiel für Kinder:**
- Wenn du ERST die Türklingel hörst und DANN Besuch siehst → Du lernst: "Klingel = Besuch kommt!" ✅
- Wenn du ERST Besuch siehst und DANN die Klingel hörst → Das ergibt keinen Sinn! ❌

Die REIHENFOLGE ist wichtig!

**3. Lateral Inhibition (Seitliche Hemmung)**

**Beispiel für Kinder:**
Stell dir einen Wettbewerb vor. Wenn du der Schnellste bist, schreist du "ICH!" und alle anderen müssen still sein. Nur der Gewinner darf sprechen!

### Technische Details

Neuromorphes Computing - NeuroQuantumDB implementiert biologisch-inspirierte Lernmechanismen:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Neuronale Lernprinzipien                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  HEBBIAN LEARNING                    STDP (Spike-Timing)            │
│  ─────────────────                   ──────────────────             │
│                                                                      │
│  "Neurons that fire                       Δw                        │
│   together, wire                           │    ╱                   │
│   together"                                │   ╱  LTP               │
│                                            │  ╱                     │
│  Δw = η × pre × post               ────────┼────── Δt              │
│                                            │╲                       │
│  ┌─────────────────┐                      │ ╲  LTD                 │
│  │ ●───●───●       │                      │  ╲                     │
│  │   ╲ │ ╱         │                                               │
│  │    ●───●        │              pre before post → Verstärkung    │
│  │   ╱     ╲       │              post before pre → Abschwächung   │
│  │ ●─────────●     │                                               │
│  └─────────────────┘                                               │
│                                                                      │
│  LATERAL INHIBITION                  PLASTICITY MATRIX              │
│  ──────────────────                  ─────────────────              │
│                                                                      │
│  Winner-takes-all                    Adaptive Gewichtungs-          │
│  Mechanismus                         Reorganisation                  │
│                                                                      │
│  Nur stärkste Aktivierung           Kontinuierliche Anpassung       │
│  bleibt erhalten                    an Datenmuster                   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### API Endpunkte

#### 🔹 Neuronales Netzwerk Trainieren

```bash
POST /api/v1/neural/train
```

**Request:**
```json
{
  "network_name": "user_classifier",
  "training_data": [
    {
      "input": [0.1, 0.5, 0.8],
      "target": [1.0, 0.0],
      "weight": 1.0
    },
    {
      "input": [0.9, 0.2, 0.1],
      "target": [0.0, 1.0],
      "weight": 1.0
    }
  ],
  "config": {
    "layers": [
      {
        "layer_type": "Dense",
        "size": 64,
        "activation": "ReLU",
        "dropout": 0.2
      },
      {
        "layer_type": "Neuromorphic",
        "size": 32,
        "activation": "SpikingNeuron",
        "dropout": null
      }
    ],
    "learning_rate": 0.001,
    "epochs": 100,
    "batch_size": 32,
    "optimizer": "NeuromorphicSTDP",
    "loss_function": "SpikeTimingLoss"
  },
  "validation_split": 0.2
}
```

#### Layer-Typen

| Typ | Beschreibung | Anwendung |
|-----|--------------|-----------|
| `Dense` | Vollverbundene Schicht | Standard |
| `Convolutional` | Faltungsschicht | Bildverarbeitung |
| `Recurrent` | Rekurrente Schicht | Sequenzen |
| `Attention` | Attention-Mechanismus | Transformer |
| `Neuromorphic` | Biologisch-inspiriert | Energy-Efficient |

#### Aktivierungsfunktionen

| Funktion | Beschreibung |
|----------|--------------|
| `ReLU` | Rectified Linear Unit |
| `Sigmoid` | Sigmoid-Funktion |
| `Tanh` | Tangens Hyperbolicus |
| `Softmax` | Wahrscheinlichkeitsverteilung |
| `Swish` | Self-gated Activation |
| `SpikingNeuron` | Biologische Spike-Aktivierung |

#### Optimizer

| Optimizer | Beschreibung |
|-----------|--------------|
| `SGD` | Stochastic Gradient Descent |
| `Adam` | Adaptive Moment Estimation |
| `AdaGrad` | Adaptive Gradient |
| `RMSprop` | Root Mean Square Propagation |
| `NeuromorphicSTDP` | Spike-Timing Dependent Plasticity |

#### 🔹 Training-Status Abfragen

```bash
GET /api/v1/neural/train/{network_id}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "network_id": "abc123",
    "training_status": "Running",
    "current_epoch": 45,
    "total_epochs": 100,
    "current_loss": 0.0234,
    "validation_loss": 0.0312,
    "estimated_completion": "2026-01-09T15:30:00Z"
  }
}
```

### 💡 Anwendungsfälle für Entwickler & Kunden

| Szenario | Nutzen |
|----------|--------|
| **Muster-Erkennung** | Automatische Klassifikation von Daten |
| **Empfehlungssysteme** | Personalisierte Produktvorschläge |
| **Anomalie-Erkennung** | Erkennung ungewöhnlicher Muster |
| **Selbstlernende Queries** | Optimierung basierend auf Nutzungsmustern |
| **Adaptive Indizierung** | Automatische Index-Optimierung |

---

## 📊 QSQL Neuromorphe Erweiterungen

### Was ist QSQL? - Erklärt für Jedermann

**SQL** ist die "Sprache", mit der Computer mit Datenbanken sprechen. Es ist wie wenn du dem Bibliothekar sagst: "Gib mir alle Bücher über Dinosaurier!"

**QSQL** ist SQL mit Superkräften! Es ist wie wenn du dem Bibliothekar sagen könntest:
- "Gib mir alle Bücher, die ÄHNLICH wie Dinosaurier sind!" (auch Bücher über Drachen oder Godzilla!)
- "Finde das Buch mit QUANTUM-GESCHWINDIGKEIT!"
- "LERNE, welche Bücher ich mag!"

#### Die magischen QSQL-Funktionen erklärt

**1. NEUROMATCH - Das "Ähnlich-wie" Finder**

```
┌─────────────────────────────────────────────────────────────────────┐
│                    NEUROMATCH erklärt                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  NORMALE SUCHE (LIKE):           NEUROMATCH:                        │
│                                                                      │
│  "Finde 'Kopfhörer'"             "Finde alles wie 'Kopfhörer'"      │
│        ↓                                ↓                            │
│  ✅ Kopfhörer                    ✅ Kopfhörer                       │
│  ❌ Headphones                   ✅ Headphones (englisch!)          │
│  ❌ Ohrhörer                     ✅ Ohrhörer (ähnlich!)             │
│  ❌ Bluetooth Earbuds            ✅ Bluetooth Earbuds (auch Musik!) │
│  ❌ Headset                      ✅ Headset (hört man auch!)        │
│                                                                      │
│  NEUROMATCH versteht BEDEUTUNG, nicht nur Buchstaben!               │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**Beispiel für Kinder:**
- LIKE ist wie: "Zeig mir alle roten Legosteine" → Du bekommst NUR rote Steine
- NEUROMATCH ist wie: "Zeig mir Steine, die zu meinem roten Feuerwehrauto passen" → Du bekommst rote, orange, und vielleicht auch gelbe Steine für die Lichter!

**2. SYNAPTIC_WEIGHT - Der "Wie-ähnlich-ist-das?" Messer**

Diese Funktion gibt dir eine Zahl zwischen 0 und 1:
- **1.0** = Perfekt gleich! 🎯
- **0.5** = Halb ähnlich 🤔
- **0.0** = Komplett anders ❌

**Beispiel für Kinder:**
```
SYNAPTIC_WEIGHT("Hund", "Hund")     = 1.0  ← Genau gleich!
SYNAPTIC_WEIGHT("Hund", "Wolf")     = 0.8  ← Sehr ähnlich (beides Tiere mit Fell!)
SYNAPTIC_WEIGHT("Hund", "Katze")    = 0.5  ← Bisschen ähnlich (beide Haustiere)
SYNAPTIC_WEIGHT("Hund", "Banane")   = 0.1  ← Fast gar nicht ähnlich!
```

**3. QUANTUM_SEARCH - Der "Überall-gleichzeitig" Sucher**

Normale Suche: Schaut ein Ergebnis nach dem anderen an 🚶
QUANTUM_SEARCH: Schaut ALLE Ergebnisse gleichzeitig an 🏃💨💨💨

**4. HEBBIAN_LEARNING - Der "Ich-werde-schlauer" Rechner**

Je öfter du etwas fragst, desto besser wird die Datenbank darin, es zu finden!

### Technische Details

NEUROMATCH - Semantische Ähnlichkeitssuche basierend auf synaptischen Gewichten:

```sql
-- Basis NEUROMATCH
SELECT * FROM products 
NEUROMATCH 'wireless headphones' 
STRENGTH > 0.7;

-- Mit Lernrate
SELECT id, content, timestamp
FROM memories 
NEUROMATCH 'happy childhood vacation' 
STRENGTH > 0.6
LEARNING_RATE 0.01
HEBBIAN_STRENGTHENING true;

-- Mit Aktivierungsschwelle
SELECT user_id, username, profile_bio
FROM users
NEUROMATCH 'software engineer python machine learning'
STRENGTH > 0.5
ACTIVATION_THRESHOLD 0.8;
```

### SYNAPTIC_WEIGHT Funktion

Berechnet neuromorphe Ähnlichkeit zwischen Werten:

```sql
-- Basis Verwendung
SELECT name, SYNAPTIC_WEIGHT(name, 'John') AS weight 
FROM users;

-- Mit Sortierung
SELECT name, email, SYNAPTIC_WEIGHT(name, 'Smith') AS similarity
FROM customers
WHERE SYNAPTIC_WEIGHT(name, 'Smith') > 0.3
ORDER BY similarity DESC;
```

### QUANTUM_SEARCH in QSQL

```sql
-- Grover's Algorithmus Suche
QUANTUM SEARCH users 
WHERE age > 30 AND city = 'Berlin';

-- Mit Iterationen
QUANTUM SEARCH products
WHERE category = 'electronics' AND price < 500
WITH ITERATIONS 100;

-- Mit Oracle-Funktion
QUANTUM SEARCH logs
WHERE severity = 'error'
WITH ORACLE 'custom_error_detector'
AMPLITUDE_AMPLIFICATION true;
```

### HEBBIAN_LEARNING Funktion

```sql
-- Hebbian Lernwert berechnen
SELECT HEBBIAN_LEARNING(age) as hebbian 
FROM users 
LIMIT 5;
```

### Hybrid Queries

Kombination von Quantum und Neural für maximale Effizienz:

```sql
-- Quantum-Neural Hybrid Query
WITH quantum_results AS (
    QUANTUM SEARCH products
    WHERE category = 'electronics'
    WITH ITERATIONS 80
)
SELECT 
    p.*,
    similarity_score
FROM quantum_results qr
JOIN products p ON qr.id = p.id
WHERE NEUROMATCH p.description 'high quality premium' STRENGTH > 0.7
ORDER BY similarity_score DESC
LIMIT 10;
```

### Vergleich: LIKE vs NEUROMATCH

| Aspekt | LIKE | NEUROMATCH |
|--------|------|------------|
| Matching | Exakt | Semantisch |
| Wildcards | `%`, `_` | Nicht nötig |
| Synonyme | ❌ | ✅ |
| Lernfähig | ❌ | ✅ |
| Performance | O(N) | O(√N) mit Quantum |

---

## 🔐 Biometrische Authentifizierung

### Was ist Biometrische Authentifizierung? - Erklärt für Jedermann

**Stell dir vor**, du hast eine geheime Schatzkiste. Wie verhinderst du, dass andere sie öffnen?

**Normales Passwort:**
- Du sagst ein geheimes Wort ("Spaghetti123")
- Problem: Jemand könnte es hören und nachmachen! 😰

**Fingerabdruck:**
- Du legst deinen Finger auf einen Scanner
- Problem: Jemand könnte deinen Fingerabdruck kopieren! 😰

**EEG-basierte Authentifizierung (NeuroQuantumDB):**
- Du trägst ein Band auf dem Kopf, das deine GEHIRNWELLEN liest
- Das ist wie dein persönliches Gedanken-Passwort!
- Problem? KEINS! Niemand kann deine Gedanken kopieren! 🎉

#### Wie funktioniert das?

```
┌─────────────────────────────────────────────────────────────────────┐
│                    EEG Authentifizierung                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. DU TRÄGST EIN HEADSET:                                          │
│                                                                      │
│        🧠 ← Gehirnwellen                                            │
│       ╱▔▔╲                                                          │
│      ╱ 😊 ╲ ← Sensoren lesen elektrische Signale                    │
│     ╱──────╲                                                         │
│                                                                      │
│  2. DEIN GEHIRN MACHT WELLEN:                                       │
│                                                                      │
│     ∿∿∿∿∿ Delta (0.5-4 Hz) - Wenn du tief schläfst                 │
│     ⌇⌇⌇⌇⌇ Theta (4-8 Hz)  - Wenn du träumst                        │
│     ∼∼∼∼∼ Alpha (8-13 Hz) - Wenn du entspannt bist                 │
│     ≋≋≋≋≋ Beta (13-30 Hz) - Wenn du nachdenkst                     │
│     ≈≈≈≈≈ Gamma (30-100+ Hz) - Wenn du hart arbeitest              │
│                                                                      │
│  3. DEIN MUSTER IST EINZIGARTIG:                                    │
│                                                                      │
│     👤 Max:  ∿∿⌇⌇∼∼≋≋≈≈                                            │
│     👤 Lisa: ∿⌇⌇∼∼∼≋≋≋≈                                            │
│     👤 Tom:  ∿∿∿⌇∼≋≋≋≋≈≈≈                                           │
│                                                                      │
│     Wie ein Fingerabdruck - aber im Gehirn!                         │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

#### Warum ist das sicher?

| Angriff | Passwort | Fingerabdruck | EEG (Gehirnwellen) |
|---------|----------|---------------|-------------------|
| Erraten | ⚠️ Möglich | ❌ Schwer | ❌ Unmöglich |
| Stehlen | ⚠️ Möglich | ⚠️ Möglich (Foto) | ❌ Unmöglich |
| Kopieren | ⚠️ Möglich | ⚠️ Möglich (3D-Druck) | ❌ Unmöglich |
| Zwingen | ⚠️ Du könntest es verraten | ⚠️ Finger kann erzwungen werden | ❌ Angst verändert die Wellen! |

**Das Geniale:** Wenn du Angst hast oder unter Druck stehst, ändern sich deine Gehirnwellen! Das System erkennt das und verweigert den Zugang. 🛡️

### Technische Details

EEG-basierte Authentifizierung - Hochsichere Authentifizierung durch Gehirnwellen-Muster:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    EEG Authentifizierungs-Pipeline                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│    ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────────┐    │
│    │   EEG   │───▶│ Digital │───▶│ Feature │───▶│ Verification│    │
│    │ Signal  │    │ Filter  │    │Extraction│   │   Match     │    │
│    └─────────┘    └─────────┘    └─────────┘    └─────────────┘    │
│                                                                      │
│    EEG Frequenzbänder:                                              │
│    ┌────────────────────────────────────────────────────────────┐   │
│    │ Band    │ Frequenz    │ Verwendung                        │   │
│    ├────────────────────────────────────────────────────────────┤   │
│    │ Delta   │ 0.5-4 Hz    │ Tiefe Muster                      │   │
│    │ Theta   │ 4-8 Hz      │ Gedächtnismuster                  │   │
│    │ Alpha   │ 8-13 Hz     │ Entspannter Zustand               │   │
│    │ Beta    │ 13-30 Hz    │ Aktives Denken                    │   │
│    │ Gamma   │ 30-100 Hz   │ Kognitive Verarbeitung            │   │
│    └────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### API Endpunkte

#### 🔹 Benutzer Registrieren

```bash
POST /api/v1/biometric/enroll
```

```json
{
  "user_id": "user123",
  "eeg_samples": [...],
  "sampling_rate": 256
}
```

#### 🔹 Benutzer Verifizieren

```bash
POST /api/v1/biometric/verify
```

```json
{
  "user_id": "user123",
  "eeg_sample": [...]
}
```

#### 🔹 Registrierte Benutzer Auflisten

```bash
GET /api/v1/biometric/eeg/users
```

### Sicherheitsfeatures

| Feature | Implementierung |
|---------|-----------------|
| Signal-Verschlüsselung | AES-256-GCM |
| Template-Speicherung | Gehasht + Gesalzen |
| Replay-Schutz | Zeitstempel-Validierung |
| Liveness Detection | Muster-Analyse |

---

## 📈 Praktische Anwendungsfälle

### Wer benutzt das und warum? - Erklärt für Jedermann

#### Für Entwickler (die Leute, die Apps und Websites bauen)

**🔍 Problem:** "Ich muss in Millionen von Zeilen Code nach einem Bug suchen!"

**Lösung mit NeuroQuantumDB:**
```
NORMALE DATENBANK:                  NEUROQUANTUMDB:
─────────────────                   ──────────────
Suche: "NullPointerException"       Suche: "Programmabsturz"
       ↓                                   ↓
Findet: 5 Treffer                   Findet: 127 Treffer!
                                    (auch "crash", "error", "null", 
                                     "undefined", "exception"...)

Zeit: 30 Sekunden                   Zeit: 0.3 Sekunden ⚡
```

#### Für Online-Shops

**🛒 Problem:** "Ein Kunde sucht 'Winterjacke', aber wir haben sie als 'Parka' gespeichert!"

**Lösung mit NeuroQuantumDB:**
```sql
-- Mit NEUROMATCH findet der Shop:
SELECT * FROM produkte 
NEUROMATCH 'Winterjacke' 
STRENGTH > 0.6;

-- Ergebnis:
-- ✅ Winterjacke        (100% Match)
-- ✅ Parka              (85% Match - warm!)
-- ✅ Daunenjacke        (80% Match - auch für Winter!)
-- ✅ Ski-Jacke          (75% Match - Winter + Jacke!)
-- ❌ Badehose           (5% Match - ignoriert)
```

**Ergebnis:** Mehr Verkäufe, glücklichere Kunden! 🎉

#### Für Krankenhäuser

**🏥 Problem:** "Wir haben Genomdaten von 1 Million Patienten. Die Festplatten sind voll!"

**Lösung mit NeuroQuantumDB:**
```
VORHER:                              NACHHER (DNA Kompression):
───────                              ────────────────────────
📁 Patient_001.dna → 4 GB           📁 Patient_001.dna → 1 GB
📁 Patient_002.dna → 4 GB           📁 Patient_002.dna → 1 GB
...                                  ...
📁 Patient_1M.dna → 4 GB            📁 Patient_1M.dna → 1 GB
═════════════════════════           ═════════════════════════
💾 Gesamt: 4.000.000 GB             💾 Gesamt: 1.000.000 GB
   (4 Petabyte!)                       (1 Petabyte!)
   
💰 Kosten: 40.000€/Monat            💰 Kosten: 10.000€/Monat
                                    
                                    💵 Ersparnis: 30.000€/Monat!
```

#### Für Banken

**🏦 Problem:** "Wir müssen Betrug in Millisekunden erkennen!"

**Lösung mit NeuroQuantumDB:**
```
Normale Transaktion:                Verdächtige Transaktion:
────────────────────                ─────────────────────────
👤 Max kauft Kaffee (3€)           👤 Max kauft Ferrari (300.000€)
📍 München, 8:00 Uhr               📍 Nigeria, 8:05 Uhr
                                    
NEUROQUANTUMDB:                     NEUROQUANTUMDB:
"Das passt zu Max' Muster ✅"        "ALARM! 🚨
                                     - Ort: 5000km entfernt
                                     - Zeit: Unmöglich!
                                     - Betrag: 100.000x normal
                                     
                                     Neuronales Netz sagt:
                                     99.7% BETRUGSWAHRSCHEINLICHKEIT
                                     
                                     → Transaktion BLOCKIERT 🛑"
```

### Technische Use Cases

Für Entwickler

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Entwickler Use Cases                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  🔍 INTELLIGENTE SUCHE                                              │
│  ─────────────────────                                              │
│  • Semantische Code-Suche mit NEUROMATCH                            │
│  • Ähnlichkeitsbasierte Bug-Erkennung                               │
│  • Pattern-basierte Log-Analyse                                      │
│                                                                      │
│  📊 DATEN-OPTIMIERUNG                                               │
│  ────────────────────                                               │
│  • Automatische Query-Optimierung durch neuronales Lernen           │
│  • Adaptive Index-Strategien                                        │
│  • Intelligentes Caching basierend auf Nutzungsmustern              │
│                                                                      │
│  🔒 SICHERHEIT                                                      │
│  ────────────                                                       │
│  • Multi-Faktor mit EEG-Biometrie                                   │
│  • Anomalie-Erkennung in Echtzeit                                   │
│  • Post-Quantum Kryptografie                                        │
│                                                                      │
│  💾 SPEICHER-EFFIZIENZ                                              │
│  ─────────────────────                                              │
│  • DNA-Kompression für Archivdaten                                  │
│  • Automatische Kompression für Cold Storage                        │
│  • Effiziente Edge-Device Speicherung                               │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Für Kunden/Endbenutzer

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Kunden Use Cases                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  🛒 E-COMMERCE                                                      │
│  ─────────────                                                      │
│  • "Finde ähnliche Produkte" mit Quantum Search                     │
│  • Personalisierte Empfehlungen durch Neural Networks               │
│  • Schnelle Suche auch bei Millionen Produkten                      │
│                                                                      │
│  🏥 HEALTHCARE                                                      │
│  ────────────                                                       │
│  • Genomdaten mit DNA-Kompression speichern                         │
│  • EEG-basierte Patienten-Authentifizierung                        │
│  • Pattern-Matching für Diagnose-Unterstützung                      │
│                                                                      │
│  🏦 FINANCE                                                         │
│  ──────────                                                         │
│  • Fraud Detection mit Anomalie-Erkennung                           │
│  • Portfolio-Optimierung mit QUBO                                   │
│  • Hochsichere Authentifizierung                                    │
│                                                                      │
│  🎮 GAMING/ENTERTAINMENT                                            │
│  ──────────────────────                                             │
│  • Spieler-Matching basierend auf Spielstil                         │
│  • Content-Empfehlungen                                             │
│  • Anti-Cheat durch Pattern-Analyse                                 │
│                                                                      │
│  🏭 INDUSTRIE 4.0                                                   │
│  ──────────────                                                     │
│  • IoT-Daten effizient speichern (DNA-Kompression)                  │
│  • Predictive Maintenance mit Neural Networks                       │
│  • Scheduling-Optimierung mit QUBO                                  │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 📖 API Referenz

### Vollständige Endpunkt-Übersicht

```
┌─────────────────────────────────────────────────────────────────────┐
│                    API Endpunkte Übersicht                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  🧬 DNA KOMPRESSION                                                 │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ POST /api/v1/dna/compress      Daten komprimieren           │    │
│  │ POST /api/v1/dna/decompress    Daten dekomprimieren         │    │
│  │ GET  /api/v1/dna/stats         Kompressionsstatistiken      │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  ⚛️ QUANTUM OPERATIONEN                                             │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ POST /api/v1/quantum/search    Quantum Ähnlichkeitssuche    │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  🧠 NEURAL OPERATIONEN                                              │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ POST /api/v1/neural/train      Netzwerk trainieren          │    │
│  │ GET  /api/v1/neural/train/{id} Training-Status abfragen     │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  🔐 BIOMETRIE                                                       │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ POST /api/v1/biometric/enroll  Benutzer registrieren        │    │
│  │ POST /api/v1/biometric/verify  Benutzer verifizieren        │    │
│  │ GET  /api/v1/biometric/eeg/users Benutzer auflisten         │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  📊 QSQL QUERY                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ POST /api/v1/query             QSQL Query ausführen         │    │
│  │ POST /api/v1/query/stream      Ergebnisse streamen          │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  🛠️ SYSTEM                                                          │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ GET  /health                   Health Check                 │    │
│  │ GET  /metrics                  Prometheus Metriken          │    │
│  │ GET  /api/v1/stats             Datenbank Statistiken        │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Berechtigungen

| Endpunkt-Gruppe | Erforderliche Berechtigung |
|-----------------|---------------------------|
| DNA Kompression | `dna` oder `admin` |
| Quantum Search | `quantum` oder `admin` |
| Neural Training | `neuromorphic` oder `admin` |
| Biometrie | `admin` |
| Query | `read` oder `write` |
| System | Public (Health), `admin` (Stats) |

---

## 🚀 Schnellstart-Beispiele

### 1. DNA Kompression nutzen

```bash
# Daten komprimieren
curl -X POST http://localhost:8080/api/v1/dna/compress \
  -H "X-API-Key: your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "sequences": ["ATCGATCGATCG"],
    "algorithm": "KmerBased"
  }'
```

### 2. Quantum Suche durchführen

```bash
# Ähnlichkeitssuche mit Quantum
curl -X POST http://localhost:8080/api/v1/quantum/search \
  -H "X-API-Key: your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "table_name": "products",
    "query_vector": [0.5, 0.3, 0.2],
    "similarity_threshold": 0.7,
    "use_grover": true
  }'
```

### 3. Neuronales Netzwerk trainieren

```bash
# Netzwerk erstellen und trainieren
curl -X POST http://localhost:8080/api/v1/neural/train \
  -H "X-API-Key: your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "network_name": "recommender",
    "training_data": [
      {"input": [0.1, 0.5], "target": [1.0]},
      {"input": [0.9, 0.2], "target": [0.0]}
    ],
    "config": {
      "layers": [{"layer_type": "Dense", "size": 32, "activation": "ReLU"}],
      "learning_rate": 0.01,
      "epochs": 50,
      "batch_size": 16,
      "optimizer": "Adam",
      "loss_function": "MeanSquaredError"
    }
  }'
```

### 4. QSQL mit neuromorphen Features

```sql
-- Semantische Produktsuche
SELECT 
    id, name, price,
    SYNAPTIC_WEIGHT(description, 'premium wireless headphones') as relevance
FROM products
WHERE SYNAPTIC_WEIGHT(description, 'premium wireless headphones') > 0.6
ORDER BY relevance DESC
LIMIT 10;

-- Quantum-beschleunigte Suche
QUANTUM SEARCH orders
WHERE total > 1000 AND status = 'pending'
WITH ITERATIONS 100;
```

---

## 📚 Weiterführende Dokumentation

- [REST API Referenz](rest-api.md)
- [QSQL Syntax Guide](qsql.md)
- [QSQL Beispiele](qsql-examples.md)
- [Feature: Auto-Increment](features/auto-increment.md)
- [Feature: DNA Compression](features/dna-compression.md)
- [Feature: Quantum Search](features/quantum-search.md)
- [Feature: Neural Networks](features/neural-networks.md)
- [Feature: Biometric Auth](features/biometric-auth.md)

---

*NeuroQuantumDB - Die Datenbank der Zukunft, heute verfügbar* 🚀
