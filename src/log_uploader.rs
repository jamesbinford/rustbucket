use aws_sdk_s3::{Client, Error};
use aws_sdk_s3::primitives::ByteStream;
use aws_config;
use std::path::Path;
use std::convert::Infallible;
use aws_smithy_types::error::Error as SmithyError;
use tracing::error;

pub async fn upload_to_s3(file_path: &str, bucket: &str, key: &str) -> Result<(), Error> {
	let shared_config = aws_config::load_from_env().await;
	let client = Client::new(&shared_config);
	
	let body = ByteStream::from_path(Path::new(file_path))
		.await
		.map_err(|e| {
			error!("Failed to create ByteStream from file path: {}", file_path);
			Error::Unhandled(SmithyError::new(e.to_string()))
		})?;
	
	client.put_object()
		.bucket(bucket)
		.key(key)
		.body(body)
		.send()
		.await?;
	
	println!("File uploaded to S3: {}", key);
	Ok(())
}
