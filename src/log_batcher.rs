use std::time::Duration;
use tokio::time::sleep;
use crate::{config, log_collector, compressor, uploader};

pub async fn start_batching_process() {
	let interval = Duration::from_secs(config::UPLOAD_INTERVAL_SECS);
	
	loop {
		let log_file = "logs/batch.log";
		let compressed_file = "logs/batch.gz";
		
		// Collect some sample logs (replace with actual log collection in production)
		log_collector::collect_log("This is a sample log", log_file);
		
		// Compress logs
		compressor::compress_logs(log_file, compressed_file).unwrap();
		
		// Generate a unique filename
		let s3_key = format!("{}/{}", config::APP_ID, "batch.gz");
		
		// Upload compressed file
		if let Err(e) = uploader::upload_to_s3(compressed_file, config::S3_BUCKET, &s3_key).await {
			eprintln!("Failed to upload batch: {}", e);
		}
		
		// Wait until the next interval
		sleep(interval).await;
	}
}