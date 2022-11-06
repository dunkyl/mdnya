mod generated_lang;

fn main() {

    let configs = generated_lang::initialize_configs();

    for (name, config) in configs {
        println!("{}: {:?} patterns", name, config.query.pattern_count());
    }

    println!("Hello, world!");
}
