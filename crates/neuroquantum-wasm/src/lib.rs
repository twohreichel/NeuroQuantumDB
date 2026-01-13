//! # NeuroQuantumDB WebAssembly Bindings
//!
//! This crate provides WebAssembly bindings for NeuroQuantumDB, enabling
//! the neuromorphic database to run directly in web browsers.
//!
//! ## Features
//!
//! - SQL query execution in the browser
//! - In-memory storage for WASM
//! - DNA compression/decompression
//! - JavaScript-friendly API with Promises
//! - TypeScript type definitions
//!
//! ## Example Usage (JavaScript)
//!
//! ```javascript
//! import init, { NeuroQuantumDB } from 'neuroquantum-wasm';
//!
//! async function main() {
//!   await init();
//!   
//!   const db = new NeuroQuantumDB();
//!   
//!   await db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)");
//!   await db.execute("INSERT INTO users (id, name) VALUES (1, 'Alice')");
//!   
//!   const results = await db.query("SELECT * FROM users");
//!   console.log(results);
//! }
//! ```

use js_sys::{Array, Object, Reflect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Initialize panic hook for better error messages in the browser console
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Error type for WASM operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct WasmError {
    message: String,
}

#[wasm_bindgen]
impl WasmError {
    /// Get the error message
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }
}

/// Result type that can be returned from a query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResultRow {
    pub values: HashMap<String, serde_json::Value>,
}

/// Main NeuroQuantumDB WebAssembly interface
#[wasm_bindgen]
pub struct NeuroQuantumDB {
    // In-memory tables for browser usage
    tables: HashMap<String, Vec<HashMap<String, serde_json::Value>>>,
}

#[wasm_bindgen]
impl NeuroQuantumDB {
    /// Create a new NeuroQuantumDB instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<NeuroQuantumDB, JsValue> {
        console_log("🧠 Initializing NeuroQuantumDB WASM...");

        Ok(NeuroQuantumDB {
            tables: HashMap::new(),
        })
    }

    /// Execute a SQL statement (INSERT, UPDATE, DELETE, CREATE TABLE, etc.)
    /// Returns the number of affected rows
    #[wasm_bindgen]
    pub async fn execute(&mut self, sql: &str) -> Result<u32, JsValue> {
        console_log(&format!("Executing SQL: {}", sql));

        // Parse and execute SQL
        match self.execute_internal(sql) {
            Ok(rows_affected) => Ok(rows_affected),
            Err(e) => Err(JsValue::from_str(&format!("SQL execution error: {}", e))),
        }
    }

    /// Execute a SQL query (SELECT) and return results as JSON
    #[wasm_bindgen]
    pub async fn query(&self, sql: &str) -> Result<JsValue, JsValue> {
        console_log(&format!("Querying SQL: {}", sql));

        match self.query_internal(sql) {
            Ok(results) => {
                // Convert results to JavaScript array
                let array = Array::new();
                for row in results {
                    let obj = Object::new();
                    for (key, value) in row {
                        let js_val = serde_wasm_bindgen::to_value(&value).map_err(|e| {
                            JsValue::from_str(&format!("Serialization error: {}", e))
                        })?;
                        Reflect::set(&obj, &JsValue::from_str(&key), &js_val)?;
                    }
                    array.push(&obj);
                }
                Ok(array.into())
            }
            Err(e) => Err(JsValue::from_str(&format!("Query error: {}", e))),
        }
    }

    /// Compress a DNA sequence
    ///
    /// Note: This is a placeholder implementation for demonstration.
    /// For production use, integrate with the full NeuroQuantumDB DNA compressor.
    #[wasm_bindgen(js_name = compressDna)]
    pub fn compress_dna(&self, sequence: &str) -> Result<Vec<u8>, JsValue> {
        console_log(&format!(
            "Compressing DNA sequence of length: {}",
            sequence.len()
        ));

        // TODO: Integrate with neuroquantum_core::dna::QuantumDNACompressor
        // For now, return a simple representation
        Ok(sequence.as_bytes().to_vec())
    }

    /// Decompress a DNA sequence
    ///
    /// Note: This is a placeholder implementation for demonstration.
    /// For production use, integrate with the full NeuroQuantumDB DNA compressor.
    #[wasm_bindgen(js_name = decompressDna)]
    pub fn decompress_dna(&self, compressed: Vec<u8>) -> Result<String, JsValue> {
        console_log(&format!(
            "Decompressing DNA data of size: {}",
            compressed.len()
        ));

        String::from_utf8(compressed)
            .map_err(|e| JsValue::from_str(&format!("Decompression error: {}", e)))
    }

    /// Get statistics about the database
    #[wasm_bindgen]
    pub fn stats(&self) -> Result<JsValue, JsValue> {
        let mut stats = HashMap::new();
        stats.insert(
            "table_count".to_string(),
            serde_json::Value::Number(self.tables.len().into()),
        );

        let total_rows: usize = self.tables.values().map(|t| t.len()).sum();
        stats.insert(
            "total_rows".to_string(),
            serde_json::Value::Number(total_rows.into()),
        );

        serde_wasm_bindgen::to_value(&stats)
            .map_err(|e| JsValue::from_str(&format!("Stats serialization error: {}", e)))
    }

    /// Clear all data from the database
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        console_log("Clearing all database data");
        self.tables.clear();
    }
}

// Internal implementation methods
impl NeuroQuantumDB {
    /// Internal SQL execution logic
    fn execute_internal(&mut self, sql: &str) -> Result<u32, String> {
        let sql_upper = sql.trim().to_uppercase();

        // Parse CREATE TABLE
        if sql_upper.starts_with("CREATE TABLE") {
            let table_name = self.parse_table_name(&sql_upper, "CREATE TABLE")?;
            self.tables.insert(table_name, Vec::new());
            return Ok(0);
        }

        // Parse INSERT
        if sql_upper.starts_with("INSERT INTO") {
            return self.execute_insert(sql);
        }

        // Parse UPDATE
        if sql_upper.starts_with("UPDATE") {
            return self.execute_update(sql);
        }

        // Parse DELETE
        if sql_upper.starts_with("DELETE FROM") {
            return self.execute_delete(sql);
        }

        Err(format!("Unsupported SQL statement: {}", sql))
    }

    /// Internal query logic
    fn query_internal(&self, sql: &str) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        let sql_upper = sql.trim().to_uppercase();

        if !sql_upper.starts_with("SELECT") {
            return Err("Only SELECT queries are supported".to_string());
        }

        // Simple SELECT * FROM table parsing
        if let Some(from_idx) = sql_upper.find("FROM") {
            let after_from = &sql[from_idx + 4..].trim();
            let table_name = after_from
                .split_whitespace()
                .next()
                .ok_or("Missing table name")?
                .to_string();

            let table = self
                .tables
                .get(&table_name)
                .ok_or(format!("Table '{}' not found", table_name))?;

            return Ok(table.clone());
        }

        Err("Invalid SELECT query".to_string())
    }

    /// Parse table name from SQL - converts to uppercase for case-insensitive matching
    fn parse_table_name(&self, sql: &str, prefix: &str) -> Result<String, String> {
        let after_prefix = sql[prefix.len()..].trim();
        let table_name = after_prefix
            .split_whitespace()
            .next()
            .ok_or("Missing table name")?
            .to_uppercase(); // Convert to uppercase for consistency
        Ok(table_name)
    }

    /// Execute INSERT statement
    fn execute_insert(&mut self, sql: &str) -> Result<u32, String> {
        // Simple INSERT parser: INSERT INTO table (col1, col2) VALUES (val1, val2)
        let sql_upper = sql.to_uppercase();

        // Extract table name
        let into_pos = sql_upper.find("INTO").ok_or("Invalid INSERT syntax")?;
        let values_pos = sql_upper.find("VALUES").ok_or("Invalid INSERT syntax")?;

        let table_part = sql[into_pos + 4..values_pos].trim();
        let table_name = table_part
            .split('(')
            .next()
            .ok_or("Invalid table name")?
            .trim()
            .to_string();

        // Extract columns
        let cols_start = table_part.find('(').ok_or("Missing column list")?;
        let cols_end = table_part.find(')').ok_or("Missing column list")?;
        let cols: Vec<String> = table_part[cols_start + 1..cols_end]
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        // Extract values
        let values_part = sql[values_pos + 6..].trim();
        let vals_start = values_part.find('(').ok_or("Missing values")?;
        let vals_end = values_part.find(')').ok_or("Missing values")?;
        let values: Vec<String> = values_part[vals_start + 1..vals_end]
            .split(',')
            .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
            .collect();

        if cols.len() != values.len() {
            return Err("Column and value count mismatch".to_string());
        }

        // Create row
        let mut row = HashMap::new();
        for (col, val) in cols.iter().zip(values.iter()) {
            // Try to parse as number, otherwise string
            let json_val = if let Ok(num) = val.parse::<i64>() {
                serde_json::Value::Number(num.into())
            } else if let Ok(num) = val.parse::<f64>() {
                // Handle floating point values carefully
                match serde_json::Number::from_f64(num) {
                    Some(n) => serde_json::Value::Number(n),
                    None => return Err(format!("Invalid floating point value: {}", val)),
                }
            } else {
                serde_json::Value::String(val.clone())
            };
            row.insert(col.clone(), json_val);
        }

        // Insert into table
        let table = self
            .tables
            .get_mut(&table_name)
            .ok_or(format!("Table '{}' not found", table_name))?;
        table.push(row);

        Ok(1)
    }

    /// Execute UPDATE statement (Not yet implemented)
    fn execute_update(&mut self, _sql: &str) -> Result<u32, String> {
        Err("UPDATE statements are not yet implemented in the WASM version".to_string())
    }

    /// Execute DELETE statement (Not yet implemented)
    fn execute_delete(&mut self, _sql: &str) -> Result<u32, String> {
        Err("DELETE statements are not yet implemented in the WASM version".to_string())
    }
}

impl Default for NeuroQuantumDB {
    fn default() -> Self {
        Self::new().expect("Failed to create default NeuroQuantumDB instance")
    }
}

/// Log a message to the browser console
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Helper function to log messages
fn console_log(s: &str) {
    log(s);
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_create_db() {
        let db = NeuroQuantumDB::new().unwrap();
        assert_eq!(db.tables.len(), 0);
    }

    #[wasm_bindgen_test]
    fn test_create_table() {
        let mut db = NeuroQuantumDB::new().unwrap();
        let result = db.execute_internal("CREATE TABLE users (id INTEGER, name TEXT)");
        assert!(result.is_ok());
        assert!(db.tables.contains_key("USERS"));
    }

    #[wasm_bindgen_test]
    fn test_insert() {
        let mut db = NeuroQuantumDB::new().unwrap();
        db.execute_internal("CREATE TABLE users (id INTEGER, name TEXT)")
            .unwrap();
        let result = db.execute_internal("INSERT INTO users (id, name) VALUES (1, 'Alice')");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[wasm_bindgen_test]
    fn test_query() {
        let mut db = NeuroQuantumDB::new().unwrap();
        db.execute_internal("CREATE TABLE users (id INTEGER, name TEXT)")
            .unwrap();
        db.execute_internal("INSERT INTO users (id, name) VALUES (1, 'Alice')")
            .unwrap();

        let results = db.query_internal("SELECT * FROM users");
        assert!(results.is_ok());
        let rows = results.unwrap();
        assert_eq!(rows.len(), 1);
    }
}
