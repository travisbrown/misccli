use clap::Parser;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    let _ = misccli::log::init(opts.verbose);

    // Only 16-byte MD5 digests are stored, never the lines themselves.
    let mut digests = HashSet::new();

    let count_a = for_each_line(open_reader(&opts.path_a, opts.zstd)?, |line| {
        digests.insert(md5::compute(line).0);
    })?;

    log::info!("Read {} lines from {}", count_a, opts.path_a.display());

    let count_b = for_each_line(open_reader(&opts.path_b, opts.zstd)?, |line| {
        digests.remove(&md5::compute(line).0);
    })?;

    log::info!("Read {} lines from {}", count_b, opts.path_b.display());

    if !digests.is_empty() {
        log::error!(
            "Missing {} lines in {}",
            digests.len(),
            opts.path_b.display()
        );
    }

    Ok(())
}

/// Open a file as a buffered reader, optionally decompressing.
fn open_reader(path: &Path, zstd: bool) -> Result<Box<dyn BufRead>, std::io::Error> {
    let file = File::open(path)?;

    Ok(if zstd {
        Box::new(BufReader::new(zstd::stream::read::Decoder::new(file)?))
    } else {
        Box::new(BufReader::new(file))
    })
}

/// Apply an action to each line (as raw bytes, without the trailing newline)
/// and return the number of lines read.
///
/// Reading bytes with a reused buffer avoids the per-line `String` allocation
/// and UTF-8 validation that `BufRead::lines` would perform.
fn for_each_line<R: BufRead, F: FnMut(&[u8])>(
    mut reader: R,
    mut action: F,
) -> Result<u64, std::io::Error> {
    let mut buffer = Vec::new();
    let mut count = 0;

    loop {
        buffer.clear();

        if reader.read_until(b'\n', &mut buffer)? == 0 {
            return Ok(count);
        }

        let mut line = buffer.as_slice();

        // Strip the line terminator so files with and without trailing
        // newlines (or with Windows line endings) compare consistently.
        if let Some(stripped) = line.strip_suffix(b"\n") {
            line = stripped;
        }
        if let Some(stripped) = line.strip_suffix(b"\r") {
            line = stripped;
        }

        action(line);
        count += 1;
    }
}

/// Check whether the lines of the first file are a subset of the lines of the second
#[derive(Parser)]
#[clap(name = "subset", about, version, author)]
struct Opts {
    /// Level of verbosity
    #[clap(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
    /// Treat both input files as zstd-compressed
    #[clap(long)]
    zstd: bool,
    path_a: PathBuf,
    path_b: PathBuf,
}
