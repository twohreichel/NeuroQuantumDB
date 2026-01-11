# WebAssembly Integration Guide

This document provides detailed information about integrating the NeuroQuantumDB WebAssembly module into your web applications.

## Quick Start

### Using from NPM (Recommended)

```bash
npm install neuroquantum-wasm
```

```javascript
import init, { NeuroQuantumDB } from 'neuroquantum-wasm';

async function main() {
  // Initialize the WASM module
  await init();
  
  // Create a database instance
  const db = new NeuroQuantumDB();
  
  // Use the database
  await db.execute("CREATE TABLE users (id INTEGER, name TEXT)");
  await db.execute("INSERT INTO users VALUES (1, 'Alice')");
  const results = await db.query("SELECT * FROM users");
  console.log(results);
}

main();
```

### Using from CDN

```html
<script type="module">
  import init, { NeuroQuantumDB } from 'https://unpkg.com/neuroquantum-wasm/neuroquantum_wasm.js';
  
  async function main() {
    await init();
    const db = new NeuroQuantumDB();
    // Use the database...
  }
  
  main();
</script>
```

## Building from Source

### Prerequisites

- Rust 1.70 or later
- wasm-pack
- Node.js (for testing)

### Build Commands

```bash
# Development build (faster, larger)
cd crates/neuroquantum-wasm
wasm-pack build --target web --dev

# Release build (optimized, smaller)
wasm-pack build --target web --release

# For Node.js
wasm-pack build --target nodejs --release

# For bundlers (webpack, rollup, etc.)
wasm-pack build --target bundler --release
```

### Build Outputs

After building, the `pkg/` directory will contain:

- `neuroquantum_wasm.js` - JavaScript bindings
- `neuroquantum_wasm_bg.wasm` - WebAssembly binary
- `neuroquantum_wasm.d.ts` - TypeScript definitions
- `package.json` - NPM package metadata

## API Reference

### Initialization

```typescript
import init, { NeuroQuantumDB } from 'neuroquantum-wasm';

// Initialize with default WASM URL
await init();

// Or specify custom WASM path
await init('/path/to/neuroquantum_wasm_bg.wasm');
```

### NeuroQuantumDB Class

#### Constructor

```typescript
const db = new NeuroQuantumDB();
```

Creates a new in-memory database instance.

#### Methods

##### execute(sql: string): Promise<number>

Executes a SQL statement and returns the number of affected rows.

**Supported statements:**
- `CREATE TABLE table_name (col1 TYPE, col2 TYPE, ...)`
- `INSERT INTO table_name (col1, col2) VALUES (val1, val2)`

**Not yet implemented:**
- `UPDATE` statements
- `DELETE` statements
- `DROP TABLE` statements

```typescript
const rowsAffected = await db.execute(
  "CREATE TABLE products (id INTEGER, name TEXT)"
);

await db.execute(
  "INSERT INTO products (id, name) VALUES (1, 'Widget')"
);
```

##### query(sql: string): Promise<Array<Object>>

Executes a SELECT query and returns results as an array of objects.

**Supported queries:**
- `SELECT * FROM table_name`

```typescript
const results = await db.query("SELECT * FROM products");
// Returns: [{ id: 1, name: 'Widget' }]
```

##### compressDna(sequence: string): Uint8Array

Compresses a DNA sequence.

> **Note:** Currently uses a placeholder implementation. Future versions will integrate with the full NeuroQuantumDB DNA compressor.

```typescript
const compressed = db.compressDna("ATCGATCG");
```

##### decompressDna(compressed: Uint8Array): string

Decompresses a DNA sequence.

```typescript
const original = db.decompressDna(compressed);
```

##### stats(): Object

Returns database statistics.

```typescript
const stats = db.stats();
// Returns: { table_count: 1, total_rows: 5 }
```

##### clear(): void

Clears all data from the database.

```typescript
db.clear();
```

## Framework Integration

### React

```typescript
import { useEffect, useState } from 'react';
import init, { NeuroQuantumDB } from 'neuroquantum-wasm';

function App() {
  const [db, setDb] = useState<NeuroQuantumDB | null>(null);
  const [results, setResults] = useState([]);
  
  useEffect(() => {
    async function initDb() {
      await init();
      setDb(new NeuroQuantumDB());
    }
    initDb();
  }, []);
  
  async function runQuery() {
    if (!db) return;
    const data = await db.query("SELECT * FROM users");
    setResults(data);
  }
  
  return (
    <div>
      <button onClick={runQuery}>Run Query</button>
      <pre>{JSON.stringify(results, null, 2)}</pre>
    </div>
  );
}
```

### Vue 3

```vue
<template>
  <div>
    <button @click="runQuery">Run Query</button>
    <pre>{{ results }}</pre>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue';
import init, { NeuroQuantumDB } from 'neuroquantum-wasm';

const db = ref(null);
const results = ref([]);

onMounted(async () => {
  await init();
  db.value = new NeuroQuantumDB();
});

async function runQuery() {
  if (!db.value) return;
  results.value = await db.value.query("SELECT * FROM users");
}
</script>
```

### Angular

```typescript
import { Component, OnInit } from '@angular/core';
import init, { NeuroQuantumDB } from 'neuroquantum-wasm';

@Component({
  selector: 'app-database',
  template: `
    <button (click)="runQuery()">Run Query</button>
    <pre>{{ results | json }}</pre>
  `
})
export class DatabaseComponent implements OnInit {
  private db?: NeuroQuantumDB;
  results: any[] = [];
  
  async ngOnInit() {
    await init();
    this.db = new NeuroQuantumDB();
  }
  
  async runQuery() {
    if (!this.db) return;
    this.results = await this.db.query("SELECT * FROM users");
  }
}
```

## Performance Considerations

### Bundle Size

- Uncompressed WASM: ~114 KB
- Gzipped WASM: ~55 KB
- JavaScript glue: ~10-20 KB
- **Total (gzipped): ~65-75 KB**

### Memory Usage

The WASM module uses in-memory storage:
- Minimal overhead for empty database
- Memory grows with data volume
- Use `.clear()` to free memory when needed

### Best Practices

1. **Initialize once**: Call `init()` only once per page load
2. **Reuse instances**: Create a single `NeuroQuantumDB` instance and reuse it
3. **Batch operations**: Group multiple inserts into transactions (future feature)
4. **Monitor memory**: Use `stats()` to track database size

## Browser Compatibility

### Minimum Requirements

- WebAssembly support
- ES6 modules
- Promises/async-await

### Tested Browsers

- ✅ Chrome/Edge 89+
- ✅ Firefox 89+
- ✅ Safari 15+
- ✅ Opera 75+

### Polyfills

For older browsers, you may need:
- WebAssembly polyfill (for IE11, not recommended)
- ES6 transpilation (Babel)

## Error Handling

```typescript
try {
  await db.execute("INVALID SQL");
} catch (error) {
  console.error('Database error:', error);
  // Handle error appropriately
}
```

Common errors:
- SQL syntax errors
- Table not found
- Type mismatches
- Unsupported operations (UPDATE, DELETE)

## Debugging

### Enable Logging

The WASM module logs to the browser console:

```javascript
// Messages appear in browser DevTools console
const db = new NeuroQuantumDB();
// Logs: "🧠 Initializing NeuroQuantumDB WASM..."
```

### TypeScript Support

TypeScript definitions are included:

```typescript
import init, { NeuroQuantumDB } from 'neuroquantum-wasm';

// TypeScript will provide autocompletion and type checking
const db: NeuroQuantumDB = new NeuroQuantumDB();
```

## Security Considerations

### Data Privacy

- All data stored in-memory only
- No data persistence (cleared on page reload)
- No network communication
- Runs in browser sandbox

### Content Security Policy

If using CSP, ensure:

```
Content-Security-Policy: script-src 'self' 'wasm-unsafe-eval';
```

## Future Enhancements

Planned features:
- [ ] IndexedDB persistence
- [ ] Full SQL support (UPDATE, DELETE, JOIN, etc.)
- [ ] Transactions
- [ ] Indexes
- [ ] WebWorker support
- [ ] SharedArrayBuffer for multi-threading
- [ ] Full DNA compression integration
- [ ] Neuromorphic query optimization

## Troubleshooting

### WASM Module Not Loading

**Error:** `Failed to fetch WASM module`

**Solution:** Ensure WASM files are served with correct MIME type:
```
Content-Type: application/wasm
```

### CORS Issues

**Error:** `Cross-Origin Request Blocked`

**Solution:** Serve files from a web server, not `file://` protocol

### Out of Memory

**Error:** `Out of memory`

**Solution:** 
- Call `db.clear()` periodically
- Reduce data volume
- Split into smaller databases

## Contributing

To contribute to the WASM module:

1. Make changes to `crates/neuroquantum-wasm/src/lib.rs`
2. Build: `wasm-pack build --target web --dev`
3. Test in browser
4. Run tests: `cargo test --target wasm32-unknown-unknown`
5. Submit pull request

## License

MIT License - see LICENSE file

## Support

- 📖 [Documentation](https://neuroquantumdb.org/docs)
- 🐛 [Issue Tracker](https://github.com/neuroquantumdb/neuroquantumdb/issues)
- 💬 [Discussions](https://github.com/neuroquantumdb/neuroquantumdb/discussions)
