use pulldown_cmark::{html, Options, Parser};
use std::env;
use std::fs;
use std::path::Path;

// This is a build-time documentation generator script.
// Panicking on errors is acceptable as it will fail the build early.
#[allow(clippy::expect_used)]
fn main() {
    // Change to project root if we're in scripts directory
    let current_dir = env::current_dir().expect("Failed to get current directory");
    if current_dir.ends_with("scripts") {
        env::set_current_dir("..").expect("Failed to change to parent directory");
    }

    let working_dir = env::current_dir().expect("Failed to get working directory");
    println!("📂 Working directory: {}", working_dir.display());

    let docs = vec![
        (
            "docs/developer_guide.md",
            "target/doc/guides/developer_guide.html",
            "Developer Guide",
        ),
        (
            "docs/user_guide.md",
            "target/doc/guides/user_guide.html",
            "User Guide",
        ),
        (
            "docs/README.md",
            "target/doc/guides/README.html",
            "Documentation Index",
        ),
        (
            "docs/quick_reference.md",
            "target/doc/guides/quick_reference.html",
            "Quick Reference",
        ),
        (
            "docs/NAVIGATION.md",
            "target/doc/guides/NAVIGATION.html",
            "Navigation",
        ),
    ];

    // Ensure output directory exists
    if let Err(e) = fs::create_dir_all("target/doc/guides") {
        eprintln!("❌ Failed to create output directory: {e}");
        std::process::exit(1);
    }

    let mut success_count = 0;
    let mut skip_count = 0;

    for (input, output, title) in docs {
        if !Path::new(input).exists() {
            eprintln!("⚠️  Skipping {input}: file not found");
            skip_count += 1;
            continue;
        }

        println!("📄 Converting {input} to HTML...");

        let markdown = match fs::read_to_string(input) {
            | Ok(content) => content,
            | Err(e) => {
                eprintln!("❌ Failed to read {input}: {e}");
                continue;
            },
        };

        // Parse markdown with extensions
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

        let parser = Parser::new_ext(&markdown, options);

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        // Wrap in HTML template
        let full_html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link rel="stylesheet" href="docs-style.css">
</head>
<body>
    <nav style="background: linear-gradient(135deg, #667eea, #764ba2); padding: 1rem 2rem; margin-bottom: 2rem;">
        <a href="README.html" style="color: white; margin-right: 2rem;">📚 Home</a>
        <a href="developer_guide.html" style="color: white; margin-right: 2rem;">🔧 Developer</a>
        <a href="user_guide.html" style="color: white; margin-right: 2rem;">👥 User</a>
        <a href="quick_reference.html" style="color: white; margin-right: 2rem;">⚡ Quick Ref</a>
        <a href="../neuroquantum_api/index.html" style="color: white;">📖 API Docs</a>
    </nav>
    <div class="container">
        {html_output}
    </div>
    <footer style="text-align: center; padding: 2rem; margin-top: 3rem; border-top: 2px solid #e1e4e8; color: #666;">
        <p>NeuroQuantumDB v0.1.0 | MIT License | Made with 🧠</p>
    </footer>
</body>
</html>"#
        );

        match fs::write(output, full_html) {
            | Ok(()) => {
                println!("✅ Generated {output}");
                success_count += 1;
            },
            | Err(e) => {
                eprintln!("❌ Failed to write {output}: {e}");
            },
        }
    }

    // Copy CSS file
    if Path::new("docs/docs-style.css").exists() {
        match fs::copy("docs/docs-style.css", "target/doc/guides/docs-style.css") {
            | Ok(_) => println!("✅ Copied docs-style.css"),
            | Err(e) => eprintln!("⚠️  Failed to copy CSS: {e}"),
        }
    }

    println!(
        "\n📊 Summary: {success_count} converted, {skip_count} skipped"
    );

    if success_count > 0 {
        println!("✅ Documentation generation complete!");
    } else {
        eprintln!("❌ No documentation files were converted!");
        std::process::exit(1);
    }
}
