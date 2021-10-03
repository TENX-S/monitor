fn main() -> anyhow::Result<()> {
    let proto_root = std::path::PathBuf::from(dotenv::var("PROTOS_ROOT")?).join("notify");
    let proto_files = &[proto_root.join("fsevent.proto")];

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(proto_files, &[proto_root])?;

    for file in proto_files {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    Ok(())
}
