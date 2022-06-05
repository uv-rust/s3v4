 # s3v4: A library for signing S3 requests and pre-signing URLs.
 
 [reference](https://docs.aws.amazon.com/AmazonS3/latest/API/sigv4-query-string-auth.html)

 This crate provides an `s3v4::signature` function that can be used to sign a request to an S3 endpoint
 and an `s3v4::presign` function that can be used to generate a presigned URL.
 
 Examples are provided showing how to upload and download objects as well as as
 how to generate a presigned URL and retrieve information about objects and buckets.
 
 Errors are internally managed through the `error_chain` crate and can be converted to a `String`
 or accessed through the `description`, `display_chain` or `backtrace` methods in case
 a full backtrace is needed.

 # Examples
 
 ## Signing a request
 ```rust
    let signature: s3v4::Signature = s3v4::signature(
        url,
        method,
        &access,
        &secret,
        &region,
        &"s3",
        "UNSIGNED-PAYLOAD", //payload hash, or "UNSIGNED-PAYLOAD"
    ).map_err(|err| format!("Signature error: {}", err.display_chain()))?;
``` 
 
 ### Using the signature data to make a request 

 #### Hyper 
 ```rust
    let req = Request::builder()
        .method(Method::PUT)
        .header("x-amz-content-sha256", "UNSIGNED-PAYLOAD")
        .header("x-amz-date", &signature.date_time)
        .header("authorization", &signature.auth_header)
 ```
 #### Ureq
 ```rust
    let agent = AgentBuilder::new().build();
    let response = agent
        .put(&uri)
        .set("x-amz-content-sha256", "UNSIGNED-PAYLOAD")
        .set("x-amz-date", &signature.date_time)
        .set("authorization", &signature.auth_header)
 ```
 ## URL pre-sign

 ```rust
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
 ```

 The following code can be used as is to generate a presigned URL. 

 ```rust
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
 ```
 Run with 
 ```shell
 cargo run --example presign -- <endpoint URL> <access> <secret> <method> \
    <expiration in seconds> <region> ["YYYY-MM-DDTHH:MM:SSZ" (timestamp)]
 ```
 
 To send the request just use `curl` with
 * `-I` for `HEAD` requests
 * --file-upload for `PUT` requests
 * nothing for `GET` requests

## Upload

Upload example using the [::ureq] crate.


