use clap::Parser;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    let _ = misccli::logging::init(opts.verbose);

    let ids = BufReader::new(File::open(opts.ids)?)
        .lines()
        .collect::<Result<HashSet<_>, _>>()?;

    log::info!("Read {} IDs", ids.len());

    let delimiter: u8 = opts
        .delimiter
        .try_into()
        .map_err(|_| format!("delimiter is not a single byte: {}", opts.delimiter))?;

    let mut data = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(opts.headers)
        .flexible(true)
        .from_path(opts.data)?;

    let mut output = csv::WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());

    if opts.headers {
        output.write_record(data.headers()?)?;
    }

    for row in data.into_records() {
        let row = row?;
        // Rows may legitimately vary in length (the reader is flexible), but
        // every row must at least reach the ID column.
        let id_value = row.get(opts.column).ok_or_else(|| {
            format!(
                "line {} has no column {}",
                row.position().map_or(0, |position| position.line()),
                opts.column
            )
        })?;

        if opts.exclude != ids.contains(id_value) {
            output.write_record(&row)?;
        }
    }

    output.flush()?;

    Ok(())
}

/// Filter the rows of a delimited file by matching a column against a set of IDs
#[derive(Parser)]
#[clap(name = "filter", about, version, author)]
struct Opts {
    /// Level of verbosity
    #[clap(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
    /// Exclude rows whose ID is in the ID file instead of including them
    #[clap(long)]
    exclude: bool,
    /// Zero-based index of the ID column in the data file
    #[clap(long, default_value = "0")]
    column: usize,
    /// Field delimiter for the data file
    #[clap(short, long, default_value = ",")]
    delimiter: char,
    /// Treat the first row of the data file as a header
    #[clap(long)]
    headers: bool,
    /// File containing one ID per line
    ids: PathBuf,
    /// Delimited data file to filter
    data: PathBuf,
}
