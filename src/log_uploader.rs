use aws_sdk_s3::{Client, Error};
use aws_sdk_s3::types::ByteStream;

pub async fn upload_to_s3(file_path: &str, bucket: &str, key: &str) -> Result<(), Error> {
	let shared_config = aws_config::load_from_env().await;
	let client = Client::new(&shared_config);
	
	let body = ByteStream::from_path(file_path).await?;
	
	client.put_object()
		.bucket(bucket)
		.key(key)
		.body(body)
		.send()
		.await?;
	
	println!("File uploaded to S3: {}", key);
	Ok(())
}
