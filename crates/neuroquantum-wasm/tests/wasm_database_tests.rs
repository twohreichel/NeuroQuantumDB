//! WASM database tests for NeuroQuantumDB
//!
//! Tests for the main NeuroQuantumDB WASM interface including:
//! - Database creation
//! - Table creation
//! - Insert operations
//! - Query operations

use neuroquantum_wasm::NeuroQuantumDB;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_create_db() {
    let db = NeuroQuantumDB::new().unwrap();
    assert_eq!(db.table_count(), 0);
}

#[wasm_bindgen_test]
fn test_create_table() {
    let mut db = NeuroQuantumDB::new().unwrap();
    let result = db.execute_internal("CREATE TABLE users (id INTEGER, name TEXT)");
    assert!(result.is_ok());
    assert!(db.has_table("USERS"));
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
