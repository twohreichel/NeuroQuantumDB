//! Test parsing of recursive CTE queries

fn main() {
    let parser = neuroquantum_qsql::Parser::new();

    // Test 1: Working query (from unit tests)
    let sql1 = r#"
        WITH RECURSIVE hierarchy AS (
            SELECT id, name FROM employees WHERE parent_id IS NULL
            UNION ALL
            SELECT e.id, e.name FROM employees e JOIN hierarchy h ON e.parent_id = h.id
        )
        SELECT * FROM hierarchy
    "#;

    println!("Test 1: Simple query");
    match parser.parse(sql1) {
        | Ok(_) => println!("  SUCCESS\n"),
        | Err(e) => println!("  ERROR: {:?}\n", e),
    }

    // Test 2: With "1 as level"
    let sql2 = r#"
        WITH RECURSIVE hierarchy AS (
            SELECT id, name, 1 as level FROM employees WHERE parent_id IS NULL
            UNION ALL
            SELECT e.id, e.name, 2 as level FROM employees e JOIN hierarchy h ON e.parent_id = h.id
        )
        SELECT * FROM hierarchy
    "#;

    println!("Test 2: With integer alias (1 as level)");
    match parser.parse(sql2) {
        | Ok(_) => println!("  SUCCESS\n"),
        | Err(e) => println!("  ERROR: {:?}\n", e),
    }

    // Test 3: With arithmetic expression (s.level + 1)
    let sql3 = r#"
        WITH RECURSIVE hierarchy AS (
            SELECT id, name FROM employees WHERE parent_id IS NULL
            UNION ALL
            SELECT e.id, h.level + 1 FROM employees e JOIN hierarchy h ON e.parent_id = h.id
        )
        SELECT * FROM hierarchy
    "#;

    println!("Test 3: With arithmetic expression (h.level + 1)");
    match parser.parse(sql3) {
        | Ok(_) => println!("  SUCCESS\n"),
        | Err(e) => println!("  ERROR: {:?}\n", e),
    }

    // Test 4: Full query
    let sql4 = r#"
        WITH RECURSIVE subordinates AS (
            SELECT id, name, manager_id, 1 as level
            FROM employees
            WHERE manager_id IS NULL
            
            UNION ALL
            
            SELECT e.id, e.name, e.manager_id, s.level + 1
            FROM employees e
            INNER JOIN subordinates s ON e.manager_id = s.id
        )
        SELECT id, name, level FROM subordinates ORDER BY level, id
    "#;

    println!("Test 4: Full recursive CTE query");
    match parser.parse(sql4) {
        | Ok(_) => println!("  SUCCESS\n"),
        | Err(e) => println!("  ERROR: {:?}\n", e),
    }
}
