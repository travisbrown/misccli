use clap::Parser;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    let _ = misccli::log::init(opts.verbose);

    let reader_a = BufReader::new(File::open(opts.path_a)?);
    let reader_b = BufReader::new(File::open(opts.path_b)?);

    for line in misccli::merge::merge_lines(reader_a.lines(), reader_b.lines()) {
        let line = line??;

        println!("{line}");
    }

    /*let mut index_a = 0;
    let mut index_b = 0;
    let mut skipped = 0;

    let mut next_a = reader_a.next();
    let mut next_b = reader_b.next();

    loop {
        match (next_a, next_b) {
            (Some(next_a), Some(next_b)) => {}
            (Some(next_a), None) => {}
            (None, Some(next_b)) => {}
            (None, None) => {
                break;
            }
        }
    }*/

    Ok(())
}

/// Merge two sorted files, removing duplicates
#[derive(Parser)]
#[clap(name = "merge", about, version, author)]
struct Opts {
    /// Level of verbosity
    #[clap(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
    path_a: String,
    path_b: String,
}
