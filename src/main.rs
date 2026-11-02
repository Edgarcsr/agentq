use agentq::detect::{Language, scan_source_files};
use std::collections::HashMap;
use std::env;
use std::path::Path;

fn main() {
    let root = env::args().nth(1).unwrap_or_else(|| String::from("."));
    let mut files = scan_source_files(Path::new(&root));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let total = files.len() as f64;
    let mut per_language: HashMap<Language, usize> = HashMap::new();
    for (_, language) in &files {
        *per_language.entry(*language).or_default() += 1;
    }

    for (path, language) in &files {
        let percent = per_language[language] as f64 * 100.0 / total;
        println!(
            "{:<8} {:>5.1}%  {}",
            language.label(),
            percent,
            path.display()
        );
    }
    println!("\n{} arquivo(s) detectado(s) em {}", files.len(), root);
}
