//! Integration tests for SQL transaction control features

use neuroquantum_qsql::parser::QSQLParser;
use neuroquantum_qsql::ast::Statement;

#[test]
fn test_parse_transaction_statements() {
    let parser = QSQLParser::new();

    // Test BEGIN
    let result = parser.parse("BEGIN");
    assert!(result.is_ok(), "Failed to parse BEGIN");
    match result.unwrap() {
        Statement::BeginTransaction(_) => {},
        other => panic!("Expected BeginTransaction, got {:?}", other),
    }

    // Test START TRANSACTION
    let result = parser.parse("START TRANSACTION");
    assert!(result.is_ok(), "Failed to parse START TRANSACTION");
    matches!(result.unwrap(), Statement::BeginTransaction(_));

    // Test COMMIT
    let result = parser.parse("COMMIT");
    assert!(result.is_ok(), "Failed to parse COMMIT");
    matches!(result.unwrap(), Statement::Commit(_));

    // Test ROLLBACK
    let result = parser.parse("ROLLBACK");
    assert!(result.is_ok(), "Failed to parse ROLLBACK");
    matches!(result.unwrap(), Statement::Rollback(_));

    // Test SAVEPOINT
    let result = parser.parse("SAVEPOINT sp1");
    assert!(result.is_ok(), "Failed to parse SAVEPOINT");
    match result.unwrap() {
        Statement::Savepoint(sp) => {
            assert_eq!(sp.name, "sp1", "Savepoint name mismatch");
        },
        other => panic!("Expected Savepoint, got {:?}", other),
    }

    // Test ROLLBACK TO SAVEPOINT
    let result = parser.parse("ROLLBACK TO SAVEPOINT sp1");
    assert!(result.is_ok(), "Failed to parse ROLLBACK TO SAVEPOINT");
    match result.unwrap() {
        Statement::RollbackToSavepoint(rts) => {
            assert_eq!(rts.name, "sp1", "Savepoint name mismatch");
        },
        other => panic!("Expected RollbackToSavepoint, got {:?}", other),
    }

    // Test RELEASE SAVEPOINT
    let result = parser.parse("RELEASE SAVEPOINT sp1");
    assert!(result.is_ok(), "Failed to parse RELEASE SAVEPOINT");
    match result.unwrap() {
        Statement::ReleaseSavepoint(rs) => {
            assert_eq!(rs.name, "sp1", "Savepoint name mismatch");
        },
        other => panic!("Expected ReleaseSavepoint, got {:?}", other),
    }
}

#[test]
fn test_transaction_case_insensitivity() {
    let parser = QSQLParser::new();

    let test_cases = vec![
        "BEGIN",
        "begin",
        "Begin",
        "START TRANSACTION",
        "start transaction",
        "COMMIT",
        "commit",
        "ROLLBACK",
        "rollback",
        "SAVEPOINT test_sp",
        "savepoint test_sp",
    ];

    for sql in test_cases {
        let result = parser.parse(sql);
        assert!(result.is_ok(), "Failed to parse: {}", sql);
    }
}

#[test]
fn test_multiple_savepoints() {
    let parser = QSQLParser::new();

    // Test different savepoint names
    let savepoint_names = vec!["sp1", "sp2", "nested_sp", "checkpoint_1"];

    for name in savepoint_names {
        let sql = format!("SAVEPOINT {}", name);
        let result = parser.parse(&sql);
        assert!(result.is_ok(), "Failed to parse: {}", sql);
        
        match result.unwrap() {
            Statement::Savepoint(sp) => {
                assert_eq!(sp.name, name, "Savepoint name mismatch");
            },
            other => panic!("Expected Savepoint, got {:?}", other),
        }
    }
}

#[test]
fn test_transaction_workflow() {
    let parser = QSQLParser::new();

    // Simulate a typical transaction workflow
    let workflow = vec![
        "BEGIN",
        "SAVEPOINT before_insert",
        "SAVEPOINT after_validation",
        "ROLLBACK TO SAVEPOINT after_validation",
        "RELEASE SAVEPOINT before_insert",
        "COMMIT",
    ];

    for sql in workflow {
        let result = parser.parse(sql);
        assert!(result.is_ok(), "Failed to parse workflow step: {}", sql);
    }
}
