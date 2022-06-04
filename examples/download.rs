use std::fs::File;
use std::time::Instant;
use ureq::AgentBuilder;
use url;

struct RequestData {
    endpoint: url::Url,
    access: String,
    secret: String,
    bucket: String,
    key: String,
    region: String,
}
fn main() -> Result<(), String> {
    let file_name = std::env::args().nth(1).expect("missing file name");
    let endpoint =
        url::Url::parse(&std::env::args().nth(2).expect("missing url")).expect("Malformed URL");
    let bucket = std::env::args().nth(3).expect("missing bucket");
    let key = std::env::args().nth(4).expect("missing key");
    let access = std::env::var("S3_ACCESS").map_err(|err| err.to_string())?;
    let secret = std::env::var("S3_SECRET").map_err(|err| err.to_string())?;
    let region = match std::env::args().nth(5) {
        Some(r) => r,
        _ => "us-east-1".to_string(),
    };
    let start = Instant::now();
    let rd = RequestData {
        endpoint,
        access,
        secret,
        bucket,
        key,
        region,
    };
    let len = download_object(&rd, &file_name)?;
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "{:.2} s {:.2} MiB/s",
        elapsed,
        (len as f64 / 0x100000 as f64) / elapsed
    );
    Ok(())
}

//------------------------------------------------------------------------------
fn download_object(req_data: &RequestData, filename: &str) -> Result<u64, String> {
    let uri = format!(
        "{}{}/{}",
        req_data.endpoint.as_str(),
        req_data.bucket,
        req_data.key
    );

    let url = url::Url::parse(&uri).map_err(|err| err.to_string())?;
    let method = "GET";
    let signature = s3v4::signature(
        &url,
        method,
        &req_data.access,
        &req_data.secret,
        &req_data.region,
        &"s3",
        "UNSIGNED-PAYLOAD",
    )
    .map_err(|err| format!("{:?}", err))?;
    let agent = AgentBuilder::new().build();
    let response = agent
        .get(&uri)
        .set("x-amz-content-sha256", "UNSIGNED-PAYLOAD")
        .set("x-amz-date", &signature.date_time)
        .set("authorization", &signature.auth_header)
        .call()
        .map_err(|err| {
            let r = err.into_response().unwrap();
            format!("{}: {}", r.status(), r.into_string().unwrap())
        })?;
    let len = response
        .header("Content-Length")
        .ok_or("No Content-Length header in response")?
        .parse::<u64>()
        .map_err(|err| err.to_string())?;
    let mut r = response.into_reader();
    let mut f = File::create(filename).map_err(|err| err.to_string())?;
    std::io::copy(&mut r, &mut f).map_err(|err| err.to_string())?;
    Ok(len)
}
