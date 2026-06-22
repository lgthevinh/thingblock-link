use std::path::{Path, PathBuf};

fn main() {
    compile_protos();
}

/// Compile the vendored arduino-cli protos when present.
///
/// Uses `protox` (a pure-Rust proto compiler) so the build needs no system
/// `protoc`; it also bundles the `google/protobuf/*` well-known types. The
/// resulting descriptor set is handed to tonic's `compile_fds`. If `proto/`
/// holds no `.proto` files, codegen is skipped so the crate still builds.
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

    let fds = protox::compile(&protos, [proto_root]).expect("protox: compile arduino-cli protos");

    tonic_prost_build::configure()
        .build_server(false) // the helper is a gRPC client only
        .include_file("mod.rs") // one nested module tree for every proto package
        .compile_fds(fds)
        .expect("tonic: generate arduino-cli client");
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
