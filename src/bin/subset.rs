use clap::Parser;
use std::collections::HashSet;
use std::fs::File;
use std::hash::{BuildHasherDefault, Hasher};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use xxhash_rust::xxh3::xxh3_128;

/// A set of 128-bit line digests with a pass-through hasher.
type DigestSet = HashSet<u128, BuildHasherDefault<DigestHasher>>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    let _ = misccli::logging::init(opts.verbose);

    // Only 16-byte digests are stored, never the lines themselves.
    let mut digests = DigestSet::default();

    let count_a = for_each_line(open_reader(&opts.path_a, opts.zstd)?, |line| {
        digests.insert(xxh3_128(line));
    })?;

    log::info!("Read {} lines from {}", count_a, opts.path_a.display());

    let count_b = for_each_line(open_reader(&opts.path_b, opts.zstd)?, |line| {
        digests.remove(&xxh3_128(line));
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

/// A hasher that passes the low 64 bits of a digest through unchanged.
///
/// The xxHash3 digests are already uniformly distributed, so rehashing them
/// with the standard library's default SipHash would be wasted work.
#[derive(Default)]
struct DigestHasher(u64);

impl Hasher for DigestHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _bytes: &[u8]) {
        unreachable!("the digest set only hashes u128 values");
    }

    fn write_u128(&mut self, value: u128) {
        // `as` truncates to the low 64 bits, which is all a `Hasher` returns.
        self.0 = value as u64;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn collect_lines(input: &str) -> (u64, Vec<Vec<u8>>) {
        let mut lines = Vec::new();
        let count = for_each_line(Cursor::new(input), |line| lines.push(line.to_vec()))
            .expect("in-memory reads cannot fail");

        (count, lines)
    }

    #[test]
    fn for_each_line_strips_terminators() {
        let (count, lines) = collect_lines("a\r\nb\nc");

        assert_eq!(count, 3);
        assert_eq!(lines, [b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn for_each_line_keeps_interior_blank_lines() {
        let (count, lines) = collect_lines("a\n\nb\n");

        assert_eq!(count, 3);
        assert_eq!(lines, [b"a".to_vec(), b"".to_vec(), b"b".to_vec()]);
    }

    #[test]
    fn for_each_line_empty_input() {
        let (count, lines) = collect_lines("");

        assert_eq!(count, 0);
        assert!(lines.is_empty());
    }
}
