use aws_sdk_s3::{Client, Error};
use aws_sdk_s3::primitives::ByteStream;
use aws_config;
use std::path::Path;
use std::convert::Infallible;
// Removed: use aws_smithy_types::error::Unhandled as SmithyUnhandledError;
use tracing::error;

pub async fn upload_to_s3(file_path: &str, bucket: &str, key: &str) -> Result<(), Error> {
	let shared_config = aws_config::load_from_env().await;
	let client = Client::new(&shared_config);
	
	let body = ByteStream::from_path(Path::new(file_path))
		.await
		.expect("TEMPORARY: Failed to create ByteStream from path during test run. Should be replaced with proper error handling.");
	
	client.put_object()
		.bucket(bucket)
		.key(key)
		.body(body)
		.send()
		.await?;
	
	println!("File uploaded to S3: {}", key);
	Ok(())
}
