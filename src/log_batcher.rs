use std::time::Duration;
use tokio::time::sleep;
use crate::{log_collector, log_compressor, log_uploader};
use config::{Config, File};

pub async fn start_batching_process() {
	let settings = Config::builder().add_source(File::with_name("Config")).build().unwrap();
	let interval_secs: u64 = settings.get("general.upload_interval_secs").unwrap();
	let interval = Duration::from_secs(interval_secs);
	let app_id: String = settings.get("aws.app_id").unwrap();
	let s3_bucket: String = settings.get("aws.s3_bucket").unwrap();
	
	loop {
		let log_file = "logs/batch.log";
		let compressed_file = "logs/batch.gz";
		
		// Collect some sample logs (replace with actual log collection in production)
		log_collector::collect_log("This is a sample log", log_file);
		
		// Compress logs
		log_compressor::compress_logs(log_file, compressed_file).unwrap();
		
		// Generate a unique filename
		let s3_key = format!("{}/{}", app_id, "batch.gz");
		
		// Upload compressed file
		if let Err(e) = log_uploader::upload_to_s3(compressed_file, &s3_bucket, &s3_key).await {
			eprintln!("Failed to upload batch: {}", e);
		}
		
		// Wait until the next interval
		sleep(interval).await;
	}
}