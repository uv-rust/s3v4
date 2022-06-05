//! Pre-sign URLs for S3 storage requests.
//! Usage:
//!
//! ```shell
//! cargo run --example presign -- <endpoint URL> <access> <secret> <method> \
//!    <expiration in seconds> <region> ["YYYY-MM-DDTHH:MM:SSZ" (timestamp)]
//! ```
//!  
//! ## Examples
//! ### HEAD
//! ```shell
//! cargo run --example presign https://play.min.io/bucket/key $S3_ACCESS $S3_SECRET HEAD 10000 "us-east-1"
//! curl -I <url>
//! ```
//! ### GET
//! ```shell
//! cargo run --example presign https://play.min.io/bucket/key $S3_ACCESS $S3_SECRET GET 10000 "us-east-1"
//! curl <url>
//! ```
//! ### PUT
//! ```shell
//! cargo run --example presign https://play.min.io/bucket/key $S3_ACCESS $S3_SECRET PUT 10000 "us-east-1"
//! curl --upload-file <file> <url>
//! ```
//! ### PUT with metadata
//! ```shell
//! cargo run --example presign https://play.min.io/bucket/key $S3_ACCESS $S3_SECRET PUT 10000 "us-east-1"
//! curl --upload-file <file> <url> -H "x-amz-meta-foo: bar"
//! ```
//! ### GET with timestamp
//! ```shell
//! cargo run --example presign https://play.min.io/bucket/key $S3_ACCESS $S3_SECRET GET 10000 "us-east-1" "2022-06-14T00:00:00Z"
//! ```
use url;
fn main() -> Result<(), String> {
    let url =
        url::Url::parse(&std::env::args().nth(1).expect("missing url")).expect("malformed URL");
    let access = std::env::args().nth(2).expect("missing access");
    let secret = std::env::args().nth(3).expect("missing secret");
    let method = std::env::args().nth(4).expect("missing method");
    let expiration = std::env::args()
        .nth(5)
        .expect("missing expiration (seconds)")
        .parse::<u64>()
        .expect("wrong expiration format");
    let region = std::env::args().nth(6).expect("missing region");
    let service = std::env::args().nth(7).expect("missing service");
    let date_time: chrono::DateTime<chrono::Utc> = match std::env::args().nth(8) {
        Some(d) => chrono::DateTime::parse_from_rfc3339(&d)
            .expect("Invalid date format (should be \"YYYY-MM-DDTHH:MM:SSZ)\"")
            .into(),
        None => chrono::Utc::now(),
    };
    let payload_hash = "UNSIGNED-PAYLOAD";
    let pre_signed_url = s3v4::pre_signed_url(
        &access,
        &secret,
        expiration,
        &url,
        &method,
        &payload_hash,
        &region,
        &date_time,
        &service,
    )
    .map_err(|err| format!("{:?}", err))?;
    println!("{}", pre_signed_url);
    Ok(())
}
