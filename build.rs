use std::path::{Path, PathBuf};

fn main() {
    compile_protos();
}

/// Compile the vendored arduino-cli protos when present.
///
/// The protos are vendored at the gRPC milestone (see the design doc). Until
/// then `proto/` holds no `.proto` files and codegen is skipped so the crate
/// still builds.
fn compile_protos() {
    println!("cargo:rerun-if-changed=proto");

    let proto_root: &Path = Path::new("proto");
    if !proto_root.exists() {
        return;
    }

    let protos: Vec<PathBuf> = collect_protos(proto_root);
    if protos.is_empty() {
        return;
    }

    tonic_prost_build::configure()
        .build_server(false) // the helper is a gRPC client only
        .compile_protos(&protos, &[proto_root.to_path_buf()])
        .expect("failed to compile arduino-cli protos");
}

fn collect_protos(dir: &Path) -> Vec<PathBuf> {
    let mut protos: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read proto dir") {
        let path: PathBuf = entry.expect("proto dir entry").path();
        if path.is_dir() {
            protos.extend(collect_protos(&path));
        } else if path.extension().is_some_and(|ext| ext == "proto") {
            protos.push(path);
        }
    }
    protos
}
