use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    inject_lua_files();
    generate_c_header();
}

fn generate_c_header() {
    let aud_lib = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(aud_lib)
        .with_language(cbindgen::Language::C)
        .with_pragma_once(false)
        .with_include_guard("AUD_LIB_BINDINGS")
        .with_no_includes()
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("inc/aud.h");
}

fn inject_lua_files() {
    let aud_lib = env::var("CARGO_MANIFEST_DIR").unwrap();
    let prj_dir = Path::new(&aud_lib).join("../..");

    // We're injecting the .lua API and DOCS into the library
    // so that we can ensure coherency between this library
    // version and the API/DOCS. Doing this in the build
    // script allows the logic to only rerun if the API/DOCS.lua
    // actually changed which is faster than using `include_str!`
    creates_tables_from_sources(&[
        Module {
            name: "auscope",
            sources: &[
                Source {
                    name: "API",
                    path: &prj_dir.join("lua/api/auscope/api.lua"),
                },
                Source {
                    name: "DOCS",
                    path: &prj_dir.join("lua/api/auscope/docs.lua"),
                },
            ],
        },
        Module {
            name: "midimon",
            sources: &[
                Source {
                    name: "API",
                    path: &prj_dir.join("lua/api/midimon/api.lua"),
                },
                Source {
                    name: "DOCS",
                    path: &prj_dir.join("lua/api/midimon/docs.lua"),
                },
            ],
        },
    ]);
}

struct Module<'a> {
    name: &'a str,
    sources: &'a [Source<'a>],
}

struct Source<'a> {
    name: &'a str,
    path: &'a Path,
}

fn creates_tables_from_sources(modules: &[Module<'_>]) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let filename = env::var("AUD_IMPORTED_LUA_RS").unwrap();
    let mut f = File::create(Path::new(&out_dir).join(filename)).unwrap();

    for module in modules {
        writeln!(f, "pub mod {} {{", module.name).unwrap();

        for source in module.sources {
            let Ok(content) = std::fs::read_to_string(source.path) else {
                panic!("Could not read file : {}", source.path.display());
            };

            writeln!(
                f,
                "pub static {}: &'static str = r#\"{}\"#;",
                source.name, content
            )
            .unwrap();
            println!("cargo:rerun-if-changed={}", source.path.display());
        }

        writeln!(f, "}}\n").unwrap();
    }
}
