use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let schema_dir = "src/core/schema";
    let schema_file = "wal_schema.fbs";

    println!("cargo:rerun-if-changed={}/{}", schema_dir, schema_file);

    let status = Command::new("flatc")
        .args([
            "--rust",
            "-o",
            &out_dir,
            &format!("{}/{}", schema_dir, schema_file),
        ])
        .status()
        .expect("Failed to run flatc");

    if !status.success() {
        panic!("flatc failed with status: {}", status);
    }

    // Optionally copy generated file to src/ if you prefer that
    let generated = format!("{}/wal_schema_generated.rs", out_dir);
    let destination = Path::new("src/core/schema/wal_schema_generated.rs");

    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::copy(&generated, &destination).expect("Failed to copy generated Rust file");
}
