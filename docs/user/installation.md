# Installation

## Systemanforderungen

### Mindestanforderungen
- **CPU**: ARM64 (AArch64) oder x86_64
- **RAM**: 1 GB (empfohlen: 2 GB+)
- **Storage**: 100 MB freier Speicherplatz
- **OS**: Linux (Ubuntu 20.04+, Debian 11+, Raspberry Pi OS)

### Empfohlene Hardware
- **Raspberry Pi 4** (4GB+ RAM) für Edge Computing
- **ARM64 Server** für Produktionsumgebungen
- **NEON-Support** für optimale Performance

## Installation über Binaries

### Offizielle Releases

Laden Sie die neueste Version von [GitHub Releases](https://github.com/neuroquantumdb/neuroquantumdb/releases):

```bash
# Für ARM64 (Raspberry Pi)
wget https://github.com/neuroquantumdb/neuroquantumdb/releases/latest/download/neuroquantum-aarch64-unknown-linux-gnu.tar.gz
tar -xzf neuroquantum-aarch64-unknown-linux-gnu.tar.gz
sudo mv neuroquantum-api /usr/local/bin/
```

### Docker Installation

```bash
# Docker Container starten
docker run -d \
  --name neuroquantumdb \
  -p 8080:8080 \
  -v $(pwd)/data:/app/data \
  ghcr.io/neuroquantumdb/neuroquantumdb:latest
```

## Installation aus Quellcode

### Voraussetzungen

```bash
# Rust installieren (neueste stabile Version)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# ARM64 Target hinzufügen (falls Cross-Compilation gewünscht)
rustup target add aarch64-unknown-linux-gnu

# Build-Tools installieren
sudo apt update
sudo apt install build-essential pkg-config libssl-dev
```

### Repository klonen und kompilieren

```bash
# Repository klonen
git clone https://github.com/neuroquantumdb/neuroquantumdb.git
cd neuroquantumdb

# Dependencies installieren und kompilieren
make build

# Tests ausführen (optional)
make test

# Installation
sudo make install
```

## Ersteinrichtung

### Konfiguration erstellen

```bash
# Konfigurationsverzeichnis erstellen
sudo mkdir -p /etc/neuroquantumdb
sudo mkdir -p /var/lib/neuroquantumdb

# Standard-Konfiguration kopieren
sudo cp config/prod.toml /etc/neuroquantumdb/config.toml
```

### Service einrichten (systemd)

```bash
# Service-Datei erstellen
sudo tee /etc/systemd/system/neuroquantumdb.service > /dev/null <<EOF
[Unit]
Description=NeuroQuantumDB
After=network.target

[Service]
Type=simple
User=neuroquantum
Group=neuroquantum
ExecStart=/usr/local/bin/neuroquantum-api --config /etc/neuroquantumdb/config.toml
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# Service aktivieren und starten
sudo systemctl daemon-reload
sudo systemctl enable neuroquantumdb
sudo systemctl start neuroquantumdb
```

## Verifikation

### Service Status prüfen

```bash
# Service Status
sudo systemctl status neuroquantumdb

# Logs ansehen
sudo journalctl -u neuroquantumdb -f
```

### API Connectivity testen

```bash
# Health Check
curl http://localhost:8080/health

# Erwartete Antwort:
# {"status":"ok","version":"0.1.0","uptime":"..."}
```

## Deinstallation

```bash
# Service stoppen und deaktivieren
sudo systemctl stop neuroquantumdb
sudo systemctl disable neuroquantumdb

# Dateien entfernen
sudo rm /usr/local/bin/neuroquantum-api
sudo rm /etc/systemd/system/neuroquantumdb.service
sudo rm -rf /etc/neuroquantumdb
sudo rm -rf /var/lib/neuroquantumdb

# Systemd neu laden
sudo systemctl daemon-reload
```

## Troubleshooting

### Häufige Probleme

1. **Permission Denied**: Stellen Sie sicher, dass der Service-User existiert
2. **Port bereits belegt**: Ändern Sie den Port in der Konfiguration
3. **NEON nicht verfügbar**: Kompilieren Sie ohne NEON-Optimierungen

### Log-Analyse

```bash
# Detaillierte Logs
RUST_LOG=debug neuroquantum-api --config config.toml

# Performance Metrics
curl http://localhost:8080/metrics
```

## Nächste Schritte

Nach erfolgreicher Installation:
1. [Schnellstart Guide](./quickstart.md) durchlesen
2. [Grundlegende Konzepte](./concepts.md) verstehen
3. [Konfiguration](./configuration.md) anpassen
