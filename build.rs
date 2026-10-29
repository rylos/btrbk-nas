//! Compiles every src/lang/<code>.json (the NAS's messages in a language
//! other than Italian and English) into the binary, so adding a language
//! is adding a file.

use std::fmt::Write;

fn main() {
    println!("cargo:rerun-if-changed=src/lang");
    let mut names: Vec<String> = std::fs::read_dir("src/lang")
        .map(|dir| {
            dir.flatten().filter_map(|e| e.file_name().into_string().ok()).filter(|n| n.ends_with(".json")).collect()
        })
        .unwrap_or_default();
    names.sort();
    let mut out = String::from("pub static TABLES: &[(&str, &str)] = &[\n");
    for name in names {
        let code = name.trim_end_matches(".json");
        let path = format!("{}/src/lang/{name}", std::env::var("CARGO_MANIFEST_DIR").unwrap());
        writeln!(out, "    ({code:?}, include_str!({path:?})),").unwrap();
    }
    out.push_str("];\n");
    std::fs::write(format!("{}/lang_tables.rs", std::env::var("OUT_DIR").unwrap()), out).unwrap();
}
