use ureq::AgentBuilder;
use url;

struct RequestData {
    endpoint: url::Url,
    access: String,
    secret: String,
    region: String,
}
fn main() -> Result<(), String> {
    let endpoint =
        url::Url::parse(&std::env::args().nth(1).expect("missing url")).expect("Malformed URL");
    let access = std::env::var("S3_ACCESS").map_err(|err| err.to_string())?;
    let secret = std::env::var("S3_SECRET").map_err(|err| err.to_string())?;
    let region = match std::env::args().nth(5) {
        Some(r) => r,
        _ => "us-east-1".to_string(),
    };
    let rd = RequestData {
        endpoint,
        access,
        secret,
        region,
    };
    let response = head(&rd)?;
    println!("{}", response);
    Ok(())
}

//------------------------------------------------------------------------------
fn head(req_data: &RequestData) -> Result<String, String> {
    let url = &req_data.endpoint;
    let method = "GET";
    let signature = s3v4::signature(
        url,
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
        .get(&url.to_string())
        .set("x-amz-content-sha256", "UNSIGNED-PAYLOAD")
        .set("x-amz-date", &signature.date_time)
        .set("authorization", &signature.auth_header)
        .call()
        .map_err(|err| {
            let r = err.into_response().unwrap();
            format!("{}: {}", r.status(), r.into_string().unwrap())
        })?;
    Ok(response.into_string().map_err(|err| format!("{:?}", err))?)
}
