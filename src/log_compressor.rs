use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{self, Read};

pub fn compress_logs(input_file: &str, output_file: &str) -> io::Result<()> {
	let input = File::open(input_file)?;
	let mut encoder = GzEncoder::new(File::create(output_file)?, Compression::default());
	io::copy(&mut input.take(10_000_000), &mut encoder)?;
	encoder.finish()?;
	Ok(())
}
