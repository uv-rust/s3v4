//! Retrieve information about a bucket or object.
//! Bucket and object name must be added to the S3 service endpoint.
//! This example uses the `ureq` crate to make a `HEAD` request, printing the response to `stdout`.
//! Credentials are read from the environment variables S3_ACCESS and S3_SECRET.
//! Bucket and object names must be included in the endpoint URL.
//! Usage:
//! ```shell
//! $ S3_ACCESS=<access> S3_SECRET=<secret> cargo run --example head \
//!    -- <endpoint URL> <region>
//! ```
use error_chain::ChainedError;
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
    let region = std::env::args().nth(2).expect("missing region");
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
    let method = "HEAD";
    let signature = s3v4::signature(
        url,
        method,
        &req_data.access,
        &req_data.secret,
        &req_data.region,
        &"s3",
        "UNSIGNED-PAYLOAD",
    ).map_err(|err| format!("Signature error: {}", err.display_chain()))?;
    let agent = AgentBuilder::new().build();
    let response = agent
        .head(&url.to_string())
        .set("x-amz-content-sha256", "UNSIGNED-PAYLOAD")
        .set("x-amz-date", &signature.date_time)
        .set("authorization", &signature.auth_header)
        .call()
        .map_err(|err| {
            let dc = format!("{}", err.to_string());
            match err.into_response() {
                Some(r) => {
                    let status = r.status();
                    let rs = r.into_string().map_err(|err| err.to_string());
                    return format!("{}: {:?}", status, rs);
                }
                None => {
                    return dc;
                }
            }
        })?;
    let headers = response
                  .headers_names()
                  .iter()
                  .filter_map(|hn| 
                    if let Some(h) = response.header(hn) {
                        Some(hn.to_string() + ":" + &h)
                    } else {
                        None 
                    })
                  .collect::<Vec<_>>()
                  .join("\n");
    Ok(headers)
}
