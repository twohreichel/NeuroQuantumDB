//! Fuzz target for QSQL Parser
//!
//! This fuzz target tests the QSQL parser with arbitrary input to find
//! parsing bugs, crashes, and edge cases.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neuroquantum_qsql::parser::{ParserConfig, QSQLParser};

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, handling invalid UTF-8 gracefully
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return, // Skip invalid UTF-8
    };

    // Skip empty inputs
    if input.is_empty() {
        return;
    }

    // Test with default parser configuration
    let parser = QSQLParser::new();

    // The parser should never panic on any input
    // It should either return Ok with a valid AST or Err with a proper error
    let _ = parser.parse(input);

    // Also test with different parser configurations
    let configs = [
        ParserConfig {
            enable_neuromorphic_extensions: true,
            enable_quantum_extensions: true,
            enable_natural_language: false,
            case_sensitive: false,
            max_query_depth: 10,
            max_tokens: 10000,
            timeout_ms: 1000,
        },
        ParserConfig {
            enable_neuromorphic_extensions: false,
            enable_quantum_extensions: false,
            enable_natural_language: false,
            case_sensitive: true,
            max_query_depth: 5,
            max_tokens: 1000,
            timeout_ms: 500,
        },
    ];

    for config in configs {
        if let Ok(parser) = QSQLParser::with_config(config) {
            let _ = parser.parse(input);
        }
    }
});
