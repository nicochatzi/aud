use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    generate_c_header();
    inject_lua_scripts();
}

fn generate_c_header() {
    const HEADER_PATH: &str = "include/aud.h";

    cbindgen::Builder::new()
        .with_crate(env::var("CARGO_MANIFEST_DIR").unwrap())
        .with_language(cbindgen::Language::C)
        .with_pragma_once(false)
        .with_include_guard("AUD_LIB_BINDINGS")
        .with_no_includes()
        .generate()
        .expect("Unable to generate C header")
        .write_to_file(HEADER_PATH);

    println!("cargo:rerun-if-changed={HEADER_PATH}");
}

/// We're injecting the .lua API and DOCS into the library
/// so that we can ensure coherency between this library
/// version and the API/DOCS. Doing this in the build
/// script allows the logic to only rerun if the API/DOCS.lua
/// actually changed which is faster than using `include_str!`
fn inject_lua_scripts() {
    let prj_dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let out_dir = Path::new(&env::var("OUT_DIR").unwrap()).join(env!("AUD_IMPORTED_LUA_RS"));
    let mut file = File::create(out_dir).unwrap();

    for cmd in ["auscope", "midimon"] {
        writeln!(file, "pub mod {} {{", cmd).unwrap();

        let apis = prj_dir.join(format!("lua/api/{cmd}/api.lua"));
        inject_script(&mut file, "API", &apis);

        let docs = prj_dir.join(format!("lua/api/{cmd}/docs.lua"));
        inject_script(&mut file, "DOCS", &docs);

        writeln!(file, "}}\n").unwrap();
    }
}

fn inject_script(file: &mut File, name: &str, path: &Path) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|_| format!("Could not read file : {}", path.display()));

    writeln!(file, "pub const {}: &str = r#\"{}\"#;", name, content).unwrap();

    println!("cargo:rerun-if-changed={}", path.display());
}
