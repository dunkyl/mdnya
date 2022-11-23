use std::{collections::HashMap, path::PathBuf, error::Error};

use hlconfig_pregen::{generated_lang, generate_hlconfig};

fn main() -> Result<(), Box<dyn Error>> {

    let start_time = std::time::Instant::now();

    let configs: HashMap<_, tree_sitter_highlight::HighlightConfiguration> = generated_lang::initialize_configs();

    let end_configure = std::time::Instant::now();

    println!("{}", std::mem::size_of::<*const hlconfig_pregen::ts_types::c_types::TSQuery>());
    println!("{}", std::mem::size_of::<[u8; std::mem::size_of::<*const hlconfig_pregen::ts_types::c_types::TSQuery>()]>());
    println!("{}", std::mem::size_of::<std::ptr::NonNull<hlconfig_pregen::ts_types::c_types::TSQuery>>());
    println!("{}", std::mem::size_of::<&'static hlconfig_pregen::ts_types::c_types::TSQuery>());

    println!("Languages and highlights load time: {:?}", end_configure - start_time);

    for (name, config) in configs {
        println!("{}: {:?} patterns", name, config.query.pattern_count());

        let config_data = generate_hlconfig(name, config);

        let output_dir = ["pregen"].iter().collect::<PathBuf>();
        if !output_dir.exists() {
            std::fs::create_dir(&output_dir)?;
        }
        let output_file_path = [output_dir, format!("{}.hlconfig", name).into()].iter().collect::<PathBuf>();
        let output_file = std::fs::File::create(output_file_path)?;

        bincode::serialize_into(output_file, &config_data)?;

        // let enc = bincode::serialize(&config_data)?;
        // let dec = bincode::deserialize::<PregeneratedHLConfig>(&enc)?;

        // let (_name, dec_conf) = hlconfig_pregen::load_hlconfig(&enc, generated_lang::language_rust())?;

        // // let ts_query = bincode::deserialize::<c_types::TSQuery>(&dec.query_data)?;

        // // println!("ts query wildcards {}", ts_query.wildcard_root_pattern_count);

        // // println!("ts pattern_maps count {}, {}", ts_query.pattern_map.size, ts_query.pattern_map.capacity);

        // // println!("{}: {:?}", dec.name, dec.regexes);

        // println!("{}", dec_conf.query.pattern_count());

    }
    

    Ok(())

}
