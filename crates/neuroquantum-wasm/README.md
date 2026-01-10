# NeuroQuantumDB WebAssembly

WebAssembly bindings for NeuroQuantumDB, enabling the neuromorphic database to run directly in web browsers.

## Features

- 🧠 **SQL Query Execution**: Run SQL queries directly in the browser
- 💾 **In-Memory Storage**: Fast, browser-based data storage
- 🧬 **DNA Compression**: Compress and decompress DNA sequences
- ⚡ **High Performance**: Compiled to WebAssembly for near-native speed
- 📦 **Small Bundle Size**: Optimized for web delivery
- 🔒 **Type-Safe**: TypeScript definitions included

## Installation

### NPM

```bash
npm install neuroquantum-wasm
```

### Using from CDN

```html
<script type="module">
  import init, { NeuroQuantumDB } from 'https://unpkg.com/neuroquantum-wasm/neuroquantum_wasm.js';
  
  await init();
  const db = new NeuroQuantumDB();
</script>
```

## Usage

### Basic SQL Operations

```javascript
import init, { NeuroQuantumDB } from 'neuroquantum-wasm';

async function main() {
  // Initialize the WASM module
  await init();
  
  // Create a new database instance
  const db = new NeuroQuantumDB();
  
  // Create a table
  await db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)");
  
  // Insert data
  await db.execute("INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.com')");
  await db.execute("INSERT INTO users (id, name, email) VALUES (2, 'Bob', 'bob@example.com')");
  
  // Query data
  const results = await db.query("SELECT * FROM users");
  console.log(results);
  // Output: [
  //   { id: 1, name: 'Alice', email: 'alice@example.com' },
  //   { id: 2, name: 'Bob', email: 'bob@example.com' }
  // ]
  
  // Get database statistics
  const stats = db.stats();
  console.log(stats);
  // Output: { table_count: 1, total_rows: 2 }
}

main();
```

### DNA Compression

```javascript
import init, { NeuroQuantumDB } from 'neuroquantum-wasm';

async function compressDNA() {
  await init();
  const db = new NeuroQuantumDB();
  
  const dnaSequence = "ATCGATCGATCGATCG";
  const compressed = db.compressDna(dnaSequence);
  console.log("Compressed size:", compressed.length);
  
  const decompressed = db.decompressDna(compressed);
  console.log("Original:", dnaSequence);
  console.log("Decompressed:", decompressed);
}
```

## API Reference

### `NeuroQuantumDB`

The main database class.

#### Constructor

```typescript
new NeuroQuantumDB(): NeuroQuantumDB
```

Creates a new in-memory database instance.

#### Methods

##### `execute(sql: string): Promise<number>`

Executes a SQL statement (INSERT, UPDATE, DELETE, CREATE TABLE, etc.).

- **Parameters:**
  - `sql`: The SQL statement to execute
- **Returns:** Promise resolving to the number of affected rows
- **Throws:** Error if the SQL is invalid

**Example:**
```javascript
await db.execute("CREATE TABLE products (id INTEGER, name TEXT)");
const rowsAffected = await db.execute("INSERT INTO products VALUES (1, 'Widget')");
```

##### `query(sql: string): Promise<Array<Object>>`

Executes a SQL SELECT query and returns the results.

- **Parameters:**
  - `sql`: The SELECT query to execute
- **Returns:** Promise resolving to an array of result objects
- **Throws:** Error if the query is invalid

**Example:**
```javascript
const results = await db.query("SELECT * FROM products WHERE id = 1");
```

##### `compressDna(sequence: string): Uint8Array`

Compresses a DNA sequence.

- **Parameters:**
  - `sequence`: DNA sequence string (e.g., "ATCGATCG")
- **Returns:** Compressed data as Uint8Array
- **Throws:** Error if compression fails

##### `decompressDna(compressed: Uint8Array): string`

Decompresses a DNA sequence.

- **Parameters:**
  - `compressed`: Compressed DNA data
- **Returns:** Original DNA sequence string
- **Throws:** Error if decompression fails

##### `stats(): Object`

Returns statistics about the database.

- **Returns:** Object containing:
  - `table_count`: Number of tables
  - `total_rows`: Total number of rows across all tables

##### `clear(): void`

Clears all data from the database.

## Building from Source

### Prerequisites

- Rust 1.70 or later
- wasm-pack

### Build Steps

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
cd crates/neuroquantum-wasm
wasm-pack build --target web --release

# Build for Node.js
wasm-pack build --target nodejs --release

# Build for bundlers (webpack, rollup, etc.)
wasm-pack build --target bundler --release
```

The built package will be in the `pkg/` directory.

## Size Optimization

The WASM module is optimized for size:

- Compiled with `opt-level = "z"` (optimize for size)
- Link-time optimization (LTO) enabled
- Debug symbols stripped
- wasm-opt used for additional optimization

Typical bundle sizes:
- Gzipped WASM: ~56 KB (release build)
- Uncompressed WASM: ~115 KB
- JavaScript glue code: ~10-20 KB

Total optimized bundle: **~66-76 KB gzipped**

> **Note:** Uses default WASM allocator for optimal security (wee_alloc removed as it's unmaintained).

## Browser Compatibility

Tested and working in:
- ✅ Chrome/Edge 89+
- ✅ Firefox 89+
- ✅ Safari 15+
- ✅ Opera 75+

## Performance

Performance characteristics in browser:
- Query execution: <1ms for simple queries
- DNA compression: ~10-50 MB/s
- Memory usage: Minimal overhead, in-memory storage only

## Limitations

Current limitations of the WASM version:

- **In-memory only**: No persistent storage (data lost on page reload)
- **Single-threaded**: No multi-threading support in WASM
- **Limited features**: Some advanced NeuroQuantumDB features not available
- **No file I/O**: Cannot read/write files directly

## Future Enhancements

Planned improvements:
- IndexedDB persistence
- More complete SQL support
- Neuromorphic query optimization
- Quantum search algorithms
- SharedArrayBuffer for multi-threading

## Examples

See the `examples/` directory for more examples:
- `browser-demo/` - Interactive web demo
- `sql-console/` - SQL console application
- `dna-compression/` - DNA compression demo
- `benchmarks/` - Performance benchmarks

## License

MIT License - see LICENSE file for details

## Contributing

Contributions welcome! Please see the main repository's CONTRIBUTING.md for guidelines.

## Support

- 📖 [Documentation](https://neuroquantumdb.org/docs)
- 🐛 [Issue Tracker](https://github.com/neuroquantumdb/neuroquantumdb/issues)
- 💬 [Discussions](https://github.com/neuroquantumdb/neuroquantumdb/discussions)
