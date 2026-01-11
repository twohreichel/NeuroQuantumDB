# NeuroQuantumDB WASM Examples

This directory contains examples demonstrating the use of NeuroQuantumDB in the browser via WebAssembly.

## Browser Demo

Open `browser-demo.html` in your web browser to see an interactive demonstration of NeuroQuantumDB running entirely in the browser.

### Features Demonstrated:

1. **SQL Console**
   - Create tables
   - Insert data
   - Query data with SELECT
   - Interactive SQL execution

2. **DNA Compression**
   - Compress DNA sequences (A, T, C, G)
   - Decompress DNA data
   - View compression statistics

3. **Statistics**
   - View database metrics
   - Track table and row counts
   - Monitor database status

### Running the Demo

#### Option 1: Direct File Access (Simple)

Simply open `browser-demo.html` directly in your browser. The demo uses a mock implementation for demonstration purposes.

#### Option 2: With Real WASM Module (Production)

To use the actual WASM module:

1. Build the WASM package:
   ```bash
   cd crates/neuroquantum-wasm
   wasm-pack build --target web --release
   ```

2. Serve the files with a local web server (required for WASM loading):
   ```bash
   # Using Python
   python3 -m http.server 8000
   
   # Or using Node.js
   npx http-server -p 8000
   ```

3. Update the import in `browser-demo.html`:
   ```javascript
   // Change this line:
   // import init, { NeuroQuantumDB } from '../pkg/neuroquantum_wasm.js';
   // To:
   import init, { NeuroQuantumDB } from '../pkg/neuroquantum_wasm.js';
   
   // And uncomment these lines in initDB():
   await init();
   db = new NeuroQuantumDB();
   // Comment out: db = new MockNeuroQuantumDB();
   ```

4. Open `http://localhost:8000/crates/neuroquantum-wasm/examples/browser-demo.html`

### Supported SQL Commands

The demo supports a simplified SQL dialect:

- `CREATE TABLE table_name (col1 TYPE, col2 TYPE, ...)`
- `INSERT INTO table_name (col1, col2) VALUES (val1, val2)`
- `SELECT * FROM table_name`

### Browser Compatibility

The demo works in:
- ✅ Chrome/Edge 89+
- ✅ Firefox 89+
- ✅ Safari 15+
- ✅ Opera 75+

### Example Queries

Try these example queries in the SQL console:

```sql
-- Create a users table
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)

-- Insert some data
INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.com')
INSERT INTO users (id, name, email) VALUES (2, 'Bob', 'bob@example.com')

-- Query the data
SELECT * FROM users
```

### DNA Compression Example

Try compressing this DNA sequence:
```
ATCGATCGATCGATCGATCGATCGATCGATCG
```

The demo will show:
- Original size in bytes
- Compressed size
- Compression ratio
- Processing time
- Verification status

## Future Examples

Planned additional examples:
- Batch operations demo
- Performance benchmarking suite
- Integration with popular frameworks (React, Vue, Angular)
- Offline-first PWA with IndexedDB persistence
- Neural network query optimization demo

## Contributing

To add new examples:

1. Create a new HTML file in this directory
2. Import the WASM module from `../pkg/`
3. Document your example in this README
4. Submit a pull request

## Support

For issues or questions:
- 📖 [Main Documentation](https://neuroquantumdb.org/docs)
- 🐛 [Issue Tracker](https://github.com/neuroquantumdb/neuroquantumdb/issues)
- 💬 [Discussions](https://github.com/neuroquantumdb/neuroquantumdb/discussions)
