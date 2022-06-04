use url;
//HEAD
//cargo run --example presign <endpoint/<bucket>> <access> <secret> HEAD 10000 "us-east-1"
//curl -I <url>
//GET
//cargo run --example presign <endpoint/<bucket/key>> <access> <secret> GET 10000 "us-east-1"
//curl <url>
//PUT
//cargo run --example presign <endpoint/<bucket/key>> <access> <secret> PUT 10000 "us-east-1"
//curl --upload-file <file> <url>
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
    let payload_hash = "UNSIGNED-PAYLOAD";
    let date_time = chrono::Utc::now();
    let service = "s3";
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
