//! The SVG icons in `assets/icons/{material,custom}` as `(name, svg)` pairs.
use std::io::Write;

fn main() {
    // Cargo supplies the current location when running the build script. A compiled
    // env! value can still point at the old checkout after moving a project.
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory missing");
    let root = std::path::PathBuf::from(manifest_dir).join("../../assets/icons");
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("icons.rs");
    let mut f = std::fs::File::create(out).unwrap();
    writeln!(f, "pub static ICONS: &[(&str, &str)] = &[").unwrap();
    for folder in ["material", "custom"] {
        let dir = root.join(folder);
        println!("cargo:rerun-if-changed={}", dir.display());
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".svg"))
            .collect();
        names.sort();
        for n in names {
            let path = dir.join(&n).canonicalize().unwrap();
            writeln!(f, "    ({:?}, include_str!({:?})),", n.trim_end_matches(".svg"), path.display().to_string()).unwrap();
        }
    }
    writeln!(f, "];").unwrap();
}
