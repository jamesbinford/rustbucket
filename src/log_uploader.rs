use aws_sdk_s3::{Client, Error};
use aws_sdk_s3::primitives::ByteStream;
use aws_config;
use std::path::Path;
// Removed: use std::convert::Infallible;
// Removed: use aws_smithy_types::error::Unhandled as SmithyUnhandledError;
use tracing::error;

pub async fn upload_to_s3(file_path: &str, bucket: &str, key: &str) -> Result<(), Error> {
	let shared_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
	let client = Client::new(&shared_config);
	
	let body = ByteStream::from_path(Path::new(file_path))
		.await
		.map_err(|e| -> aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::put_object::PutObjectError, aws_smithy_runtime_api::client::orchestrator::HttpResponse> {
			error!("Failed to create ByteStream from file path: {}", file_path);
			aws_sdk_s3::error::SdkError::construction_failure(Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>)
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
