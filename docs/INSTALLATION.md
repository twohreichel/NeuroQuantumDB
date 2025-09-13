# 🔧 Installation & Setup - So einfach wie Lego bauen!

## 🎯 In 5 Minuten zur ersten NeuroQuantumDB

### 📋 Was Sie brauchen (Checkliste):
- ✅ **Raspberry Pi 4** (4GB+ RAM empfohlen) oder Linux-Computer
- ✅ **Internet-Verbindung** für Downloads
- ✅ **10 Minuten Zeit** ⏰
- ✅ **Kaffee** ☕ (optional, aber empfohlen)

## 🚀 Schnellstart - Der 3-Schritte-Weg

### Schritt 1: Projekt herunterladen 📥
```bash
# 📁 Ordner erstellen und hineinwechseln
mkdir meine-neuroquantum-projekte
cd meine-neuroquantum-projekte

# 📥 NeuroQuantumDB herunterladen
git clone https://github.com/neuroquantumdb/neuroquantumdb.git
cd neuroquantumdb
```

### Schritt 2: Mit Docker starten (Einfachster Weg) 🐳
```bash
# 🔨 Docker-Image bauen (dauert 2-3 Minuten)
make docker-build

# 🚀 NeuroQuantumDB starten
make docker-run
```

**Das war's!** 🎉 Ihre NeuroQuantumDB läuft jetzt!

### Schritt 3: Testen ob alles funktioniert ✅
```bash
# 🧪 Schneller Systemcheck
make test-quick

# 📊 Sollte so aussehen:
✅ Neuromorphic Core: OK
✅ Quantum Engine: OK  
✅ DNA Compression: OK
✅ API Server: Running on port 8080
```

## 🏗️ Alternative: Lokale Installation

### Für Entwickler und Bastler

#### 1. Abhängigkeiten installieren
```bash
# 🦀 Rust installieren (falls noch nicht vorhanden)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 🎯 ARM64 Target hinzufügen (für Raspberry Pi)
rustup target add aarch64-unknown-linux-gnu

# 🛠️ Zusätzliche Tools
sudo apt update && sudo apt install -y \
    build-essential \
    cmake \
    git \
    python3-dev
```

#### 2. NeuroQuantumDB kompilieren
```bash
# 🏗️ Für Ihr aktuelles System bauen
make build-release

# 💪 Oder speziell für Raspberry Pi 4 (ARM64)
make build-arm64
```

#### 3. Konfiguration anpassen
```bash
# 📝 Basis-Konfiguration kopieren
cp config/dev.toml config/meine-config.toml

# ✏️ Mit Ihrem Lieblings-Editor bearbeiten
nano config/meine-config.toml
```

## ⚙️ Konfiguration verstehen

### 📄 Die wichtigsten Einstellungen erklärt:

```toml
# config/dev.toml - Wie ein Rezeptbuch für NeuroQuantumDB

[server]
host = "0.0.0.0"        # Lauscht auf allen Netzwerk-Interfaces
port = 8080             # Port 8080 (wie eine Hausnummer)
workers = 4             # 4 parallele Arbeiter (wie 4 Köche in der Küche)

[neuromorphic]
learning_rate = 0.01    # Wie schnell das System lernt (0.01 = langsam aber sicher)
plasticity_threshold = 0.5  # Wann Verbindungen sich ändern
max_synapses = 1000000  # Maximum an "Gehirn-Verbindungen"

[quantum]
grover_iterations = 10  # Anzahl Quantensuche-Durchläufe
annealing_steps = 1000  # Optimierungsschritte
parallel_queries = true # Parallele Abfragen aktiviert

[dna]
compression_level = 9   # Höchste Kompression (9 = maximum)
error_correction = true # Fehlerkorrektur ein (immer empfohlen!)
cache_size = "64MB"     # Zwischenspeicher-Größe
```

## 🔍 System-Check - Ist alles bereit?

### Hardware prüfen:
```bash
# 💾 Speicher checken (mindestens 4GB empfohlen)
free -h

# 🧠 CPU-Architektur prüfen
uname -m  # Sollte "aarch64" zeigen (Raspberry Pi 4)

# 🌡️ Temperatur überwachen (unter 80°C halten)
vcgencmd measure_temp
```

### Performance-Test:
```bash
# 🏃‍♂️ Schnelle Tests
make benchmark

# Erwartete Ergebnisse:
📊 Query Response Time: <1μs  ✅
💾 Memory Usage: <100MB     ✅  
🔋 Power Consumption: <2W   ✅
📦 Container Size: <15MB    ✅
```

## 🎨 Erste Schritte - Ihre erste Abfrage!

### Mit dem Demo-Client:
```python
# 🐍 Python-Demo starten
python3 demo_client.py

# Beispiel-Ausgabe:
🧠 NeuroQuantumDB Demo Client
✅ Verbindung hergestellt
🔍 Teste Neuromorphe Suche...
⚛️ Teste Quantum-Optimierung...
🧬 Teste DNA-Kompression...

Alle Tests erfolgreich! 🎉
Response Time: 0.8μs
Compression Ratio: 1247:1
```

### Mit QSQL (der intelligenten Abfragesprache):
```sql
-- 🧠 Ihre erste neuromorphe Abfrage
NEUROMATCH users 
WHERE age > 25 
WITH SYNAPTIC_WEIGHT 0.8;

-- ⚛️ Mit Quantum-Power
QUANTUM_SELECT products 
FROM inventory 
WHERE price < 100
WITH GROVER_ITERATIONS 15;
```

## 🐳 Docker-Optionen

### Standard-Container:
```bash
# 🏃‍♂️ Einfach starten
docker run -p 8080:8080 neuroquantumdb/core:latest
```

### Mit eigener Konfiguration:
```bash
# 📝 Mit Ihrer Konfiguration
docker run -p 8080:8080 \
  -v $(pwd)/config:/app/config \
  neuroquantumdb/core:latest
```

### Mit Persistent Storage:
```bash
# 💾 Daten bleiben erhalten
docker run -p 8080:8080 \
  -v neuroquantum-data:/app/data \
  neuroquantumdb/core:latest
```

## 🚨 Troubleshooting - Wenn mal was nicht klappt

### Problem: "Port 8080 already in use"
```bash
# 🔍 Wer nutzt den Port?
sudo netstat -tulpn | grep 8080

# 🛑 Anderen Service stoppen oder anderen Port nutzen
docker run -p 8081:8080 neuroquantumdb/core:latest
```

### Problem: "Out of memory"
```bash
# 💾 Mehr Swap-Speicher erstellen
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### Problem: "Too hot" (Raspberry Pi überhitzt)
```bash
# 🌡️ Temperatur prüfen
vcgencmd measure_temp

# 🧊 Kühlung verbessern:
# - Lüfter installieren
# - Gehäuse mit Kühlkörper verwenden  
# - CPU-Frequenz reduzieren
```

### Problem: "Build failed"
```bash
# 🔧 Abhängigkeiten neu installieren
make clean
cargo update
make build-release
```

## ✅ Installation erfolgreich!

**Gratulation!** 🎉 Sie haben NeuroQuantumDB erfolgreich installiert!

### Was Sie jetzt haben:
- ✅ Funktionsfähige NeuroQuantumDB
- ✅ API-Server auf Port 8080
- ✅ Alle drei Superhelden-Technologien aktiv
- ✅ Demo-Client zum Testen

### Nächste Schritte:
1. 📖 **[Entwickler-Guide](ENTWICKLER_GUIDE.md)** - Erste Programmierung
2. 🎯 **[QSQL Tutorial](BENUTZER_HANDBUCH.md)** - Die intelligente Abfragesprache
3. 🚀 **[API-Dokumentation](API_DOKUMENTATION.md)** - REST-API nutzen

---

> **💡 Pro-Tipp:** Speichern Sie Ihre Konfiguration in Git - so können Sie Änderungen nachverfolgen!

> **🆘 Hilfe benötigt?** Schauen Sie in die [FAQ](FAQ.md) oder erstellen Sie ein Issue auf GitHub!
