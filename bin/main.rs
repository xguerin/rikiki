use clap::Parser;
use rikiki::*;

#[derive(Parser)]
struct Arguments {
    #[arg(short, long)]
    script: String,
}

fn main() {
    let args = Arguments::parse();
    /*
     * Allocate the lisp interpreter.
     */
    let mut slab = Slab::default();
    let lisp = Lisp::new(&mut slab).unwrap();
    /*
     * Load the file.
     */
    let _ = lisp.load(&args.script);
}
