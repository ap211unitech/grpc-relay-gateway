fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::compile_protos("./contracts/users.proto")?;
    tonic_prost_build::compile_protos("./contracts/posts.proto")?;
    Ok(())
}
