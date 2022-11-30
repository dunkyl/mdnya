use std::path::PathBuf;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    Ok(
        bincode::serialize_into(
            std::fs::File::create(
                PathBuf::from(std::env::var("OUT_DIR")?).join("rust.hlconfig")
            )?, 
            &mdnya_hl_rust_gen::get_config_data()
        )?
    )
}
