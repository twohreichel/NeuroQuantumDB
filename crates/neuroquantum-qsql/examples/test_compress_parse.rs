use neuroquantum_qsql::Parser;

fn main() {
    let parser = Parser::new();
    let sql = "COMPRESS TABLE logs USING DNA";

    match parser.parse(sql) {
        | Ok(statement) => {
            println!("Success! Parsed statement: {statement:?}");
        },
        | Err(e) => {
            eprintln!("Error parsing: {e:?}");
        },
    }
}
