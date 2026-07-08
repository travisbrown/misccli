use clap::Parser;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    let _ = misccli::logging::init(opts.verbose);

    let reader_a = BufReader::new(File::open(opts.path_a)?);
    let reader_b = BufReader::new(File::open(opts.path_b)?);

    // Lock and buffer stdout once instead of paying for both on every line.
    let mut output = BufWriter::new(std::io::stdout().lock());

    for line in misccli::merge::merge_lines(reader_a.lines(), reader_b.lines()) {
        let line = line??;

        if !pipe_open(writeln!(output, "{line}"))? {
            return Ok(());
        }
    }

    pipe_open(output.flush())?;

    Ok(())
}

/// Whether output can continue: `false` means the consumer closed the pipe.
///
/// Rust ignores `SIGPIPE`, so when a downstream tool like `head` exits early,
/// writes fail with a broken-pipe error instead of killing the process; that
/// is normal shutdown for a pipeline, not a failure.
fn pipe_open(result: std::io::Result<()>) -> Result<bool, std::io::Error> {
    match result {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(false),
        Err(error) => Err(error),
    }
}

/// Merge two sorted files, removing duplicates
#[derive(Parser)]
#[clap(name = "merge", about, version, author)]
struct Opts {
    /// Level of verbosity
    #[clap(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
    path_a: PathBuf,
    path_b: PathBuf,
}
