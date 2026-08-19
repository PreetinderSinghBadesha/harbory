fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Point prost at the vendored protoc binary so building this crate
    // doesn't require protoc to be installed on the host.
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);

    tonic_build::configure().compile_protos(&["proto/harbory.proto"], &["proto"])?;

    Ok(())
}
