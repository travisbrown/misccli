use clap::Parser;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    let _ = misccli::log::init(opts.verbose);

    let ids = BufReader::new(File::open(opts.ids)?)
        .lines()
        .collect::<Result<HashSet<_>, _>>()
        .expect("Error reading IDs");

    let delimiter: u8 = opts.delimiter.try_into().expect("Invalid delimiter");

    let mut data = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(opts.headers)
        .flexible(true)
        .from_path(opts.data)
        .expect("Error opening data file");

    let mut output = csv::WriterBuilder::new()
        .has_headers(opts.headers)
        .flexible(true)
        .from_writer(std::io::stdout());

    if opts.headers {
        output
            .write_record(data.headers().expect("Invalid data file headers"))
            .expect("Error writing output");
    }

    for row in data.into_records() {
        let row = row.expect("Error reading data file");
        let id_value = row.get(opts.column).expect("Invalid record");

        if opts.exclude != ids.contains(id_value) {
            output.write_record(&row).expect("Error writing output");
        }
    }

    output.flush().expect("Error writing output");

    Ok(())
}

/// Merge two sorted files, removing duplicates
#[derive(Parser)]
#[clap(name = "filter", about, version, author)]
struct Opts {
    /// Level of verbosity
    #[clap(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
    /// By default the filter includes lines where the ID is in the ID file.
    #[clap(long)]
    exclude: bool,
    #[clap(long)]
    ids: PathBuf,
    #[clap(long)]
    data: PathBuf,
    #[clap(long, default_value = "0")]
    column: usize,
    #[clap(short, long, default_value = ",")]
    delimiter: char,
    #[clap(short)]
    headers: bool,
}
