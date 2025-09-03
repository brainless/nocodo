use std::path::Path;

fn main() {
    // Only generate types if bindings directory exists
    let bindings_dir = Path::new("bindings");
    if bindings_dir.exists() {
        println!("cargo:rerun-if-changed=src/");
        
        // This will be triggered by ts-rs macros during compilation
        // The #[ts(export)] attributes will generate TypeScript files
    }
}