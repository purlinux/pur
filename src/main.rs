use clap::Parser;
mod repo;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long, value_parser)]
    install: String,

    #[clap(short, long, value_parser)]
    delete: String,
}

fn main() {
    let args = Args::parse();

    println!("Hello, world!");
}
