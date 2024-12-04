use std::fs::OpenOptions;
use std::io::Write;

pub fn collect_log(log_message: &str, log_file: &str) {
	let mut file = OpenOptions::new()
		.create(true)
		.append(true)
		.open(log_file)
		.unwrap();
	
	writeln!(file, "{}", log_message).unwrap();
}
