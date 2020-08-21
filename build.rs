/// Compile Protos in build-time

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto_tmp/rusk.proto")?;
    Ok(())
}
