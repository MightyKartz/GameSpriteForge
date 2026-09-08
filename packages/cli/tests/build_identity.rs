// Exercise the dependency-free build script in normal Cargo test runs too.
#[allow(dead_code)]
#[path = "../build.rs"]
mod build_script;
