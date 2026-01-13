//! Fuzz target for QSQL Functions
//!
//! This fuzz target tests the QSQL-specific functions including NEUROMATCH,
//! QUANTUM_SEARCH, and Hebbian learning features with various parameter combinations.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neuroquantum_qsql::parser::{ParserConfig, QSQLParser};

/// Generate QSQL function queries from fuzz data
fn generate_qsql_queries(data: &[u8]) -> Vec<String> {
    let mut queries = Vec::new();

    // Extract parameters from fuzz data
    if data.len() < 4 {
        return queries;
    }

    let table_name = format!("table_{}", data[0] % 26);
    let field_name = format!("field_{}", data[1] % 10);
    let pattern = if data.len() > 10 {
        String::from_utf8_lossy(&data[2..10.min(data.len())]).to_string()
    } else {
        "pattern".to_string()
    };

    // Weight values from fuzz data
    let weight = (data.get(2).copied().unwrap_or(50) as f32) / 100.0;
    let threshold = (data.get(3).copied().unwrap_or(30) as f32) / 100.0;

    // NEUROMATCH queries
    queries.push(format!(
        "SELECT * FROM {} NEUROMATCH('{}')",
        table_name, pattern
    ));

    queries.push(format!(
        "SELECT {} FROM {} NEUROMATCH('{}') WHERE {} > 0",
        field_name, table_name, pattern, field_name
    ));

    queries.push(format!(
        "NEUROMATCH {} PATTERN '{}' WITH SYNAPTIC_WEIGHT {} LEARNING_RATE {}",
        table_name, pattern, weight, threshold
    ));

    queries.push(format!(
        "NEUROMATCH {} PATTERN '{}' HEBBIAN STRENGTHENING",
        table_name, pattern
    ));

    // QUANTUM_SEARCH queries
    queries.push(format!(
        "QUANTUM_SEARCH {} WHERE {} = '{}' AMPLITUDE_AMPLIFICATION",
        table_name, field_name, pattern
    ));

    queries.push(format!(
        "QUANTUM_SEARCH {} WHERE {} LIKE '%{}%' MAX_ITERATIONS {}",
        table_name,
        field_name,
        pattern,
        (data.get(4).copied().unwrap_or(10) % 100) + 1
    ));

    // QUANTUM JOIN queries
    queries.push(format!(
        "SELECT * FROM {} QUANTUM JOIN table_b ON {}.id ENTANGLE table_b.id",
        table_name, table_name
    ));

    // SUPERPOSITION queries
    queries.push(format!(
        "SUPERPOSITION (SELECT * FROM {} WHERE {} > 0) COHERENCE MAINTAIN",
        table_name, field_name
    ));

    // Hebbian learning queries
    queries.push(format!(
        "ADAPT WEIGHTS ON {} USING HEBBIAN RULE RATE {}",
        table_name, weight
    ));

    queries.push(format!(
        "LEARN PATTERN '{}' FROM {} USING HEBBIAN_LEARNING",
        pattern, table_name
    ));

    queries.push(format!(
        "LEARN PATTERN '{}' FROM {} USING STDP",
        pattern, table_name
    ));

    // Synaptic optimize queries
    queries.push(format!(
        "SYNAPTIC OPTIMIZE {} WITH PLASTICITY_THRESHOLD {}",
        table_name, threshold
    ));

    // Complex combined queries
    queries.push(format!(
        "SELECT {} FROM {} NEUROMATCH('{}') WHERE {} > 0 WITH SYNAPTIC_WEIGHT {} QUANTUM_PARALLEL",
        field_name, table_name, pattern, field_name, weight
    ));

    // Edge cases with special characters
    if let Ok(pattern_str) = std::str::from_utf8(data) {
        // Only use printable ASCII for pattern to avoid parser issues
        let safe_pattern: String = pattern_str
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ' ')
            .take(50)
            .collect();

        if !safe_pattern.is_empty() {
            queries.push(format!(
                "SELECT * FROM {} NEUROMATCH('{}')",
                table_name, safe_pattern
            ));

            queries.push(format!(
                "QUANTUM_SEARCH {} WHERE content LIKE '%{}%'",
                table_name, safe_pattern
            ));
        }
    }

    queries
}

fuzz_target!(|data: &[u8]| {
    // Skip very small inputs
    if data.is_empty() {
        return;
    }

    // Create parser with neuromorphic and quantum extensions enabled
    let config = ParserConfig {
        enable_neuromorphic_extensions: true,
        enable_quantum_extensions: true,
        enable_natural_language: false,
        case_sensitive: false,
        max_query_depth: 20,
        max_tokens: 50000,
        timeout_ms: 5000,
    };

    let parser = match QSQLParser::with_config(config.clone()) {
        Ok(p) => p,
        Err(_) => return,
    };

    // Generate and test QSQL-specific queries
    let queries = generate_qsql_queries(data);
    for query in queries {
        // The parser should never panic on any input
        let _ = parser.parse(&query);
    }

    // Also test raw input as QSQL
    if let Ok(input) = std::str::from_utf8(data) {
        // Test with QSQL prefixes to trigger neuromorphic/quantum code paths
        let qsql_prefixes = [
            "NEUROMATCH ",
            "QUANTUM_SEARCH ",
            "QUANTUM JOIN ",
            "SUPERPOSITION ",
            "LEARN PATTERN ",
            "ADAPT WEIGHTS ",
            "SYNAPTIC OPTIMIZE ",
            "ENTANGLE ",
            "SELECT * FROM users NEUROMATCH(",
            "SELECT * FROM data QUANTUM_PARALLEL ",
        ];

        for prefix in qsql_prefixes {
            let modified = format!("{}{}", prefix, input);
            let _ = parser.parse(&modified);
        }

        // Test with neuromorphic clauses
        let neuro_suffixes = [
            " WITH SYNAPTIC_WEIGHT 0.5",
            " HEBBIAN STRENGTHENING",
            " PLASTICITY_THRESHOLD 0.3",
            " QUANTUM_PARALLEL",
            " AMPLITUDE_AMPLIFICATION",
            " COHERENCE MAINTAIN",
        ];

        for suffix in neuro_suffixes {
            let modified = format!("SELECT * FROM test {}{}", input, suffix);
            let _ = parser.parse(&modified);
        }
    }

    // Test with different parser configurations
    let configs = [
        ParserConfig {
            enable_neuromorphic_extensions: true,
            enable_quantum_extensions: false,
            enable_natural_language: false,
            case_sensitive: true,
            max_query_depth: 5,
            max_tokens: 1000,
            timeout_ms: 500,
        },
        ParserConfig {
            enable_neuromorphic_extensions: false,
            enable_quantum_extensions: true,
            enable_natural_language: false,
            case_sensitive: false,
            max_query_depth: 10,
            max_tokens: 5000,
            timeout_ms: 1000,
        },
    ];

    for cfg in configs {
        if let Ok(p) = QSQLParser::with_config(cfg) {
            for query in generate_qsql_queries(data).iter() {
                let _ = p.parse(query);
            }
        }
    }
});
