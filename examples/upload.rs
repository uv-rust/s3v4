use std::fs::File;
use std::time::Instant;
use ureq::{AgentBuilder};
use url;
type HeaderMap = std::collections::HashMap<String, String>;

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
    let headers = match std::env::args().nth(6) {
        Some(h) => parse_headers(&h),
        None => HeaderMap::new(),
    };
    let len = std::fs::metadata(&file_name)
        .map_err(|err| err.to_string())?
        .len();
    if len > 0x40000000 {
        // 1GB
        return Err("File too large".to_string());
    }
    let mut file = File::open(&file_name).map_err(|err| err.to_string())?;
    let start = Instant::now();
    let rd = RequestData {
        endpoint,
        access,
        secret,
        bucket,
        key,
        region,
    };
    let mut buffer = vec![0_u8; len as usize];
    use std::io::Read;
    file.read_exact(&mut buffer)
        .map_err(|err| err.to_string())?;
    upload_object(&buffer, &rd, &headers)?;
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "{:.2} s {:.2} MiB/s",
        elapsed,
        (len / 0x100000) as f64 / elapsed
    );

    Ok(())
}

fn parse_headers(h: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    h.split(';').into_iter().for_each(|s| {
        let (k, v) = (
            s.split(':').nth(0).expect("Missing header"),
            s.split(':').nth(1).expect("Missing header value"),
        );
        if !k.is_empty() && !v.is_empty() {
            headers.insert(k.to_string(), v.to_string());
        }
    });
    headers
}

//------------------------------------------------------------------------------
fn upload_object(buffer: &[u8], req_data: &RequestData, headers: &HeaderMap) -> Result<(), String> {
    let uri = format!(
        "{}{}/{}?",
        req_data.endpoint.as_str(),
        req_data.bucket,
        req_data.key
    );

    let url = url::Url::parse(&uri).map_err(|err| err.to_string())?;
    let method = "PUT";
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
    let mut req = agent
        .put(&uri)
        .set("x-amz-content-sha256", "UNSIGNED-PAYLOAD")
        .set("x-amz-date", &signature.date_time)
        .set("authorization", &signature.auth_header)
        .set("content-length", &buffer.len().to_string());
    for (k, v) in headers {
        req = req.set(k, v);
    }
    let response = req.send_bytes(buffer).map_err(|err| format!("{:?}", err))?;
    if response.status() >= 300 {
        let status = response.status();
        let body = response.into_string().map_err(|err| err.to_string())?;
        return Err(format!("Error - {}\n{}", status, body));
    }
    let etag = response
        .header("ETag")
        .ok_or("Missing ETag")?
        .trim_matches('"');
    println!("ETag: {}", etag);
    Ok(())
}
