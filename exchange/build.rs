use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");

    let descriptor_path =
        std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("proto_descriptor.bin");

    let builder = tonic_build::configure()
        .emit_rerun_if_changed(true)
        .file_descriptor_set_path(descriptor_path);

    let protos = glob::glob("./proto/**/*.proto")
        .expect("failed to glob for proto files")
        .filter_map(|res| res.ok())
        .map(|p| p.canonicalize().unwrap())
        .collect::<Vec<_>>();

    for path in &protos {
        if path.exists() {
            println!(
                "cargo:rerun-if-changed={proto}",
                proto = path.canonicalize().unwrap().display()
            );
        }
    }

    let proto_path = PathBuf::from("./proto").canonicalize().unwrap();

    builder
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&protos, &[proto_path])?;

    Ok(())
}
