//! Fuzz target for QSQL Tokenizer
//!
//! This fuzz target specifically tests the tokenization phase of the QSQL parser
//! to ensure robust handling of malformed input at the lexical level.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neuroquantum_qsql::parser::QSQLParser;

fuzz_target!(|data: &[u8]| {
    // Test with various byte patterns to stress the tokenizer
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    if input.is_empty() {
        return;
    }

    let parser = QSQLParser::new();

    // Test parsing which internally tokenizes first
    // Focus on edge cases that might break tokenization
    let _ = parser.parse(input);

    // Test with SQL-like prefixes to trigger different code paths
    let test_prefixes = [
        "SELECT ",
        "INSERT INTO ",
        "UPDATE ",
        "DELETE FROM ",
        "CREATE TABLE ",
        "DROP TABLE ",
        "NEURAL ",
        "QUANTUM ",
        "SYNAPTIC ",
        "ENTANGLE ",
    ];

    for prefix in test_prefixes {
        let modified = format!("{}{}", prefix, input);
        let _ = parser.parse(&modified);
    }

    // Test with special characters that might confuse tokenizer
    let special_chars = ["'", "\"", "`", "--", "/*", "*/", "\\", "\0", "\n", "\r\n"];
    for special in special_chars {
        let modified = format!("{}{}{}", special, input, special);
        let _ = parser.parse(&modified);
    }
});
