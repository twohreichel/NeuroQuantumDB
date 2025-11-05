# 🚀 NeuroQuantumDB Setup Commands with `cargo run`

## Prerequisites
The system runs with `cargo run`. To execute CLI commands, use:
```bash
cargo run -p neuroquantum-api -- [COMMAND]
```

---

## 📝 Available Commands

### 1. Initialize Database (Interactive)
```bash
cargo run -p neuroquantum-api -- init
```

### 2. Initialize Database (Non-interactive with custom settings)
```bash
cargo run -p neuroquantum-api -- init --name admin --expiry-hours 8760 --output .env --yes
```

**Parameters:**
- `--name admin`: Name of the admin key
- `--expiry-hours 8760`: Validity in hours (8760 = 1 year)
- `--output .env`: Output file for the admin key
- `--yes` or `-y`: Skips confirmation prompts

### 3. Generate JWT Secret
```bash
cargo run -p neuroquantum-api -- generate-jwt-secret --output config/jwt-secret.txt
```

**Or output directly in terminal:**
```bash
cargo run -p neuroquantum-api -- generate-jwt-secret
```

### 4. Start Server (Default)
```bash
cargo run -p neuroquantum-api
# or explicitly:
cargo run -p neuroquantum-api -- serve
```

### 5. With Custom Configuration
```bash
cargo run -p neuroquantum-api -- --config config/prod.toml serve
```

---

## 🔧 Complete Setup Example

```bash
# 1. Generate JWT secret
cargo run -p neuroquantum-api -- generate-jwt-secret --output config/jwt-secret.txt

# 2. Initialize database
cargo run -p neuroquantum-api -- init --name admin --expiry-hours 8760 --output .env --yes

# 3. Start server
cargo run -p neuroquantum-api
```

---

## 💡 Helpful Tips

### Display help for a specific command:
```bash
cargo run -p neuroquantum-api -- init --help
cargo run -p neuroquantum-api -- generate-jwt-secret --help
```

### Display all available commands:
```bash
cargo run -p neuroquantum-api -- --help
```

### With release build (faster):
```bash
cargo run -p neuroquantum-api --release -- init
```

---

## 📋 Quick Reference Table

| Action | Command |
|--------|---------|
| Init (interactive) | `cargo run -p neuroquantum-api -- init` |
| Init (auto) | `cargo run -p neuroquantum-api -- init --name admin --expiry-hours 8760 --output .env --yes` |
| JWT Secret | `cargo run -p neuroquantum-api -- generate-jwt-secret --output config/jwt-secret.txt` |
| Start Server | `cargo run -p neuroquantum-api` |
| Help | `cargo run -p neuroquantum-api -- --help` |

---

## 🎯 After Setup

After successful initialization:
1. The admin key is saved in the specified file (e.g., `.env`)
2. The JWT secret is stored in `config/jwt-secret.txt`
3. You can start the server and use the API

**Important:** Keep the admin key and JWT secret secure!

