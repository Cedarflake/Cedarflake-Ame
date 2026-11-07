use prost::Message;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let descriptor = std::fs::read("build/onnx-schema/descriptor.bin")?;
    let descriptor = prost_types::FileDescriptorSet::decode(descriptor.as_slice())?;
    let output = "build/onnx-schema/generated";
    std::fs::create_dir_all(output)?;
    prost_build::Config::new()
        .out_dir(output)
        .compile_fds(descriptor)?;
    Ok(())
}
