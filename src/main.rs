use quantum_agent::detect::{Language, scan_source_files};
use std::env;
use std::path::Path;

fn main() {
    let root = env::args().nth(1).unwrap_or_else(|| String::from("."));
    let mut files = scan_source_files(Path::new(&root));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    for (path, language) in &files {
        let name = match language {
            Language::Python => "Python",
            Language::Java => "Java",
            Language::JavaScriptTypeScript => "JS/TS",
            Language::Go => "Go",
        };
        println!("{:<8} {}", name, path.display());
    }
    println!("\n{} arquivo(s) detectado(s) em {}", files.len(), root);
}
