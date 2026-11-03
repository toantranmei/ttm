//! Exposes the target triple to `ttm self-update`, which downloads the matching release archive.

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=TTM_TARGET={target}");
    println!("cargo:rerun-if-changed=build.rs");
}
