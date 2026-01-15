//! QSQL Function Performance Benchmarks
//!
//! This module benchmarks QSQL-specific functions including:
//! - NEUROMATCH for neuromorphic pattern matching
//! - QUANTUM_SEARCH for quantum-inspired search
//! - Comparison with traditional SQL LIKE queries
//!
//! Run with: cargo bench --features benchmarks --bench qsql_functions

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use neuroquantum_core::storage::Value;
use rand::prelude::*;
use std::collections::HashMap;
use std::hint::black_box;
use std::time::Duration;

/// Simple row structure for benchmarks (independent of actual storage Row)
/// Fields are used for benchmark data generation but not directly accessed
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct BenchmarkRow {
    id: u64,
    fields: HashMap<String, Value>,
}

/// Generate test data for benchmarks
fn generate_test_rows(count: usize) -> Vec<BenchmarkRow> {
    let mut rng = StdRng::seed_from_u64(42);
    let names = [
        "John", "Jane", "Alice", "Bob", "Charlie", "Diana", "Eve", "Frank",
    ];
    let domains = ["example.com", "test.org", "domain.net", "email.io"];

    (0..count)
        .map(|i| {
            let name = format!(
                "{} {} {}",
                names[rng.gen_range(0..names.len())],
                if rng.gen_bool(0.5) { "A." } else { "" },
                names[rng.gen_range(0..names.len())]
            );
            let email = format!(
                "{}{}@{}",
                name.to_lowercase().replace(' ', "."),
                i,
                domains[rng.gen_range(0..domains.len())]
            );

            let mut fields = HashMap::new();
            fields.insert("id".to_string(), Value::Integer(i as i64));
            fields.insert("name".to_string(), Value::Text(name));
            fields.insert("email".to_string(), Value::Text(email));
            fields.insert(
                "age".to_string(),
                Value::Integer(rng.gen_range(18..80) as i64),
            );
            fields.insert("score".to_string(), Value::Float(rng.gen_range(0.0..100.0)));

            BenchmarkRow {
                id: i as u64,
                fields,
            }
        })
        .collect()
}

/// Simulate NEUROMATCH function - neuromorphic pattern matching
fn neuromatch_score(text: &str, pattern: &str) -> f32 {
    if text.is_empty() || pattern.is_empty() {
        return 0.0;
    }

    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    // Exact match
    if text_lower == pattern_lower {
        return 1.0;
    }

    // Contains match
    if text_lower.contains(&pattern_lower) {
        return 0.8;
    }

    // Fuzzy match using character overlap
    let text_chars: std::collections::HashSet<char> = text_lower.chars().collect();
    let pattern_chars: std::collections::HashSet<char> = pattern_lower.chars().collect();

    let intersection = text_chars.intersection(&pattern_chars).count();
    let union = text_chars.union(&pattern_chars).count();

    if union == 0 {
        0.0
    } else {
        (intersection as f32 / union as f32) * 0.6
    }
}

/// Simulate QUANTUM_SEARCH function - quantum-inspired parallel search
fn quantum_search<'a>(
    rows: &'a [BenchmarkRow],
    column: &str,
    pattern: &str,
    threshold: f32,
) -> Vec<&'a BenchmarkRow> {
    // Quantum-inspired: evaluate all possibilities in parallel (simulated)
    rows.iter()
        .filter(|row| {
            if let Some(Value::Text(text)) = row.fields.get(column) {
                neuromatch_score(text, pattern) > threshold
            } else {
                false
            }
        })
        .collect()
}

/// Traditional SQL LIKE matching
fn sql_like_match(text: &str, pattern: &str) -> bool {
    // Simple LIKE with % wildcards
    if pattern.starts_with('%') && pattern.ends_with('%') {
        let inner = &pattern[1..pattern.len() - 1];
        text.to_lowercase().contains(&inner.to_lowercase())
    } else if let Some(suffix) = pattern.strip_prefix('%') {
        text.to_lowercase().ends_with(&suffix.to_lowercase())
    } else if let Some(prefix) = pattern.strip_suffix('%') {
        text.to_lowercase().starts_with(&prefix.to_lowercase())
    } else {
        text.to_lowercase() == pattern.to_lowercase()
    }
}

/// Benchmark NEUROMATCH function with different row counts
fn bench_neuromatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("neuromatch");
    group.measurement_time(Duration::from_secs(10));

    for row_count in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*row_count as u64));

        let rows = generate_test_rows(*row_count);

        group.bench_with_input(BenchmarkId::from_parameter(row_count), &rows, |b, rows| {
            b.iter(|| {
                let results: Vec<_> = rows
                    .iter()
                    .filter(|row| {
                        if let Some(Value::Text(name)) = row.fields.get("name") {
                            neuromatch_score(name, "John") > 0.5
                        } else {
                            false
                        }
                    })
                    .collect();
                black_box(results)
            });
        });
    }

    group.finish();
}

/// Benchmark NEUROMATCH vs SQL LIKE comparison
fn bench_neuromatch_vs_like(c: &mut Criterion) {
    let mut group = c.benchmark_group("neuromatch_vs_like");
    group.measurement_time(Duration::from_secs(10));

    for row_count in [1_000, 5_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*row_count as u64));

        let rows = generate_test_rows(*row_count);

        // NEUROMATCH
        group.bench_with_input(
            BenchmarkId::new("neuromatch", row_count),
            &rows,
            |b, rows| {
                b.iter(|| {
                    let results: Vec<_> = rows
                        .iter()
                        .filter(|row| {
                            if let Some(Value::Text(name)) = row.fields.get("name") {
                                neuromatch_score(name, "John") > 0.5
                            } else {
                                false
                            }
                        })
                        .collect();
                    black_box(results)
                });
            },
        );

        // SQL LIKE
        group.bench_with_input(BenchmarkId::new("like", row_count), &rows, |b, rows| {
            b.iter(|| {
                let results: Vec<_> = rows
                    .iter()
                    .filter(|row| {
                        if let Some(Value::Text(name)) = row.fields.get("name") {
                            sql_like_match(name, "%John%")
                        } else {
                            false
                        }
                    })
                    .collect();
                black_box(results)
            });
        });
    }

    group.finish();
}

/// Benchmark QUANTUM_SEARCH function
fn bench_quantum_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("quantum_search");
    group.measurement_time(Duration::from_secs(10));

    for row_count in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*row_count as u64));

        let rows = generate_test_rows(*row_count);

        group.bench_with_input(BenchmarkId::from_parameter(row_count), &rows, |b, rows| {
            b.iter(|| {
                let results = quantum_search(rows, "name", "Alice", 0.5);
                black_box(results)
            });
        });
    }

    group.finish();
}

/// Benchmark QUANTUM_SEARCH vs traditional linear search
fn bench_quantum_search_vs_linear(c: &mut Criterion) {
    let mut group = c.benchmark_group("quantum_vs_linear_search");
    group.measurement_time(Duration::from_secs(10));

    for row_count in [1_000, 5_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*row_count as u64));

        let rows = generate_test_rows(*row_count);

        // Quantum search
        group.bench_with_input(BenchmarkId::new("quantum", row_count), &rows, |b, rows| {
            b.iter(|| {
                let results = quantum_search(rows, "email", "example", 0.3);
                black_box(results)
            });
        });

        // Linear search with LIKE
        group.bench_with_input(BenchmarkId::new("linear", row_count), &rows, |b, rows| {
            b.iter(|| {
                let results: Vec<_> = rows
                    .iter()
                    .filter(|row| {
                        if let Some(Value::Text(email)) = row.fields.get("email") {
                            sql_like_match(email, "%example%")
                        } else {
                            false
                        }
                    })
                    .collect();
                black_box(results)
            });
        });
    }

    group.finish();
}

/// Benchmark SQL parsing performance using the QSQL parser
fn bench_sql_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("sql_parsing");
    group.measurement_time(Duration::from_secs(10));

    let queries = [
        ("simple_select", "SELECT id, name FROM users"),
        ("with_where", "SELECT * FROM users WHERE age > 18"),
        (
            "neuromatch",
            "SELECT * FROM users WHERE NEUROMATCH(name, 'John') > 0.5",
        ),
        (
            "quantum_search",
            "SELECT * FROM users WHERE QUANTUM_SEARCH(email, 'test')",
        ),
        (
            "complex_join",
            "SELECT u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id WHERE o.total > 100",
        ),
        (
            "aggregation",
            "SELECT department, AVG(salary), COUNT(*) FROM employees GROUP BY department HAVING AVG(salary) > 50000",
        ),
    ];

    for (name, query) in queries.iter() {
        group.bench_with_input(BenchmarkId::new("parse", *name), query, |b, query| {
            b.iter(|| {
                // Use the Parser from neuroquantum_qsql
                let parser = neuroquantum_qsql::Parser::new();
                black_box(parser.parse(query))
            });
        });
    }

    group.finish();
}

/// Benchmark QSQL function composition (chaining multiple functions)
fn bench_function_composition(c: &mut Criterion) {
    let mut group = c.benchmark_group("function_composition");
    group.measurement_time(Duration::from_secs(10));

    for row_count in [1_000, 5_000].iter() {
        group.throughput(Throughput::Elements(*row_count as u64));

        let rows = generate_test_rows(*row_count);

        // Single function
        group.bench_with_input(BenchmarkId::new("single", row_count), &rows, |b, rows| {
            b.iter(|| {
                let results: Vec<_> = rows
                    .iter()
                    .filter(|row| {
                        if let Some(Value::Text(name)) = row.fields.get("name") {
                            neuromatch_score(name, "John") > 0.5
                        } else {
                            false
                        }
                    })
                    .collect();
                black_box(results)
            });
        });

        // Two functions combined
        group.bench_with_input(BenchmarkId::new("double", row_count), &rows, |b, rows| {
            b.iter(|| {
                let results: Vec<_> = rows
                    .iter()
                    .filter(|row| {
                        let name_match = row
                            .fields
                            .get("name")
                            .map(|v| {
                                if let Value::Text(t) = v {
                                    neuromatch_score(t, "John") > 0.5
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false);

                        let email_match = row
                            .fields
                            .get("email")
                            .map(|v| {
                                if let Value::Text(t) = v {
                                    neuromatch_score(t, "example") > 0.3
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false);

                        name_match || email_match
                    })
                    .collect();
                black_box(results)
            });
        });

        // Three functions with ordering
        group.bench_with_input(BenchmarkId::new("triple", row_count), &rows, |b, rows| {
            b.iter(|| {
                let mut results: Vec<_> = rows
                    .iter()
                    .filter(|row| {
                        let name_match = row
                            .fields
                            .get("name")
                            .map(|v| {
                                if let Value::Text(t) = v {
                                    neuromatch_score(t, "John") > 0.3
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false);

                        let email_match = row
                            .fields
                            .get("email")
                            .map(|v| {
                                if let Value::Text(t) = v {
                                    neuromatch_score(t, "example") > 0.3
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false);

                        let age_check = row
                            .fields
                            .get("age")
                            .map(|v| {
                                if let Value::Integer(a) = v {
                                    *a > 25
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false);

                        name_match && email_match && age_check
                    })
                    .collect();

                // Sort by score
                results.sort_by(|a, b| {
                    let score_a = a
                        .fields
                        .get("score")
                        .map(|v| if let Value::Float(f) = v { *f } else { 0.0 })
                        .unwrap_or(0.0);
                    let score_b = b
                        .fields
                        .get("score")
                        .map(|v| if let Value::Float(f) = v { *f } else { 0.0 })
                        .unwrap_or(0.0);
                    score_b.partial_cmp(&score_a).unwrap()
                });

                black_box(results)
            });
        });
    }

    group.finish();
}

/// Benchmark different threshold values for NEUROMATCH
fn bench_neuromatch_thresholds(c: &mut Criterion) {
    let mut group = c.benchmark_group("neuromatch_thresholds");
    group.measurement_time(Duration::from_secs(10));

    let rows = generate_test_rows(5_000);

    for threshold in [0.3, 0.5, 0.7, 0.9].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(threshold),
            threshold,
            |b, &threshold| {
                b.iter(|| {
                    let results: Vec<_> = rows
                        .iter()
                        .filter(|row| {
                            if let Some(Value::Text(name)) = row.fields.get("name") {
                                neuromatch_score(name, "John") > threshold
                            } else {
                                false
                            }
                        })
                        .collect();
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark pattern complexity impact on NEUROMATCH
fn bench_pattern_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_complexity");
    group.measurement_time(Duration::from_secs(10));

    let rows = generate_test_rows(5_000);

    let patterns = [
        ("short", "Jo"),
        ("medium", "John"),
        ("long", "Jonathan Smith"),
        ("very_long", "Jonathan Alexander Smith Junior"),
    ];

    for (name, pattern) in patterns.iter() {
        group.bench_with_input(
            BenchmarkId::new("neuromatch", *name),
            pattern,
            |b, pattern| {
                b.iter(|| {
                    let results: Vec<_> = rows
                        .iter()
                        .filter(|row| {
                            if let Some(Value::Text(name)) = row.fields.get("name") {
                                neuromatch_score(name, pattern) > 0.3
                            } else {
                                false
                            }
                        })
                        .collect();
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_neuromatch,
    bench_neuromatch_vs_like,
    bench_quantum_search,
    bench_quantum_search_vs_linear,
    bench_sql_parsing,
    bench_function_composition,
    bench_neuromatch_thresholds,
    bench_pattern_complexity,
);

criterion_main!(benches);
