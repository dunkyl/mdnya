use std::path::PathBuf;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let output_file_path = PathBuf::from(std::env::var("OUT_DIR")?).join("rust.hlconfig");
    let output_file = std::fs::File::create(output_file_path)?;
    let config_data = mdnya_hl_rust_gen::generate_config_data()?;

    bincode::serialize_into(output_file, &config_data)?;

    Ok(())
}
