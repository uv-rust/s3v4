//! S3 v4 signing originally copied from: https://crates.io/crates/rust-s3
//! Changes:
//! 1. removed all calls to `unwrap` and repalced with Result<T, E>
//! 2. removed `anyhow`
//! 3. replaced `HashMap` with `BTreeMap` to avoid explicit sorting
//! 4. implemented `signature` function returning both signed header and time-stamp
//! 5. added functions that only use `host` and `x-amz-*` signed headers
//! 6. urlencoding is being used for encoding uris
//! 7. added function that returns a pre-signed url
/// reference: https://docs.aws.amazon.com/AmazonS3/latest/API/sigv4-query-string-auth.html
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use url::Url;
pub use urlencoding::encode as url_encode;

type HeadersMap = BTreeMap<String, String>;

type HmacSha256 = Hmac<Sha256>;

const LONG_DATETIME_FMT: &str = "%Y%m%dT%H%M%SZ";
const SHORT_DATE_FMT: &str = "%Y%m%d";

#[macro_use]
extern crate error_chain;
mod errors {
    error_chain!{}
}

use errors::*;


// -----------------------------------------------------------------------------
/// Generate a canonical query string from the query pairs in the given URL.
/// The current implementation does not support repeated keys, which should not
/// be a problem for the query string used in the request.
fn canonical_query_string(uri: &Url) -> String {
    let mut qs = BTreeMap::new();
    uri.query_pairs().for_each(|(k, v)| {
        qs.insert(
            url_encode(&k.to_string()).to_string(),
            url_encode(&v).to_string(),
        );
    });
    let kv: Vec<String> = qs.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    kv.join("&")
}

// -----------------------------------------------------------------------------
/// Generate a canonical header string using only x-amz-, host and content-lrngth headers.
fn canonical_header_string(headers: &HeadersMap) -> String {
    let key_values = headers
        .iter()
        .filter_map(|(key, value)| {
            let k = key.as_str().to_lowercase();
            if k.starts_with("x-amz-") || k == "host" {
                Some(k + ":" + value.as_str().trim())
            } else {
                None
            }
        })
        .collect::<Vec<String>>();
    key_values.join("\n")
}

// -----------------------------------------------------------------------------
/// Generate a signed header string using only x-amz-, host and content-length headers.
fn signed_header_string(headers: &HeadersMap) -> String {
    let keys = headers
        .keys()
        .filter_map(|key| {
            let k = key.as_str().to_lowercase();
            if k.starts_with("x-amz-") || k == "host" {
                Some(k)
            } else {
                None
            }
        })
        .collect::<Vec<String>>();
    keys.join(";")
}

// -----------------------------------------------------------------------------
/// Generate a canonical request.
fn canonical_request(
    method: &str,
    url: &Url,
    headers: &HeadersMap,
    payload_sha256: &str,
) -> String {
    format!(
        "{method}\n{uri}\n{query_string}\n{headers}\n\n{signed}\n{sha256}",
        method = method,
        uri = url.path().to_ascii_lowercase(),
        query_string = canonical_query_string(url),
        headers = canonical_header_string(headers),
        signed = signed_header_string(headers),
        sha256 = payload_sha256
    )
}

// -----------------------------------------------------------------------------
/// Generate an AWS scope string.
fn scope_string(date_time: &DateTime<Utc>, region: &str) -> String {
    format!(
        "{date}/{region}/s3/aws4_request",
        date = date_time.format(SHORT_DATE_FMT),
        region = region
    )
}

// -----------------------------------------------------------------------------
/// Generate the "string to sign" - the value to which the HMAC signing is
/// applied to sign requests.
fn string_to_sign(date_time: &DateTime<Utc>, region: &str, canonical_req: &str) -> String {
    let mut hasher = Sha256::default();
    hasher.update(canonical_req.as_bytes());
    let string_to = format!(
        "AWS4-HMAC-SHA256\n{timestamp}\n{scope}\n{hash}",
        timestamp = date_time.format(LONG_DATETIME_FMT),
        scope = scope_string(date_time, region),
        hash = hex::encode(hasher.finalize().as_slice())
    );
    string_to
}

// -----------------------------------------------------------------------------
/// Generate the AWS signing key, derived from the secret key, date, region,
/// and service name.
fn signing_key(
    date_time: &DateTime<Utc>,
    secret_key: &str,
    region: &str,
    service: &str,
) -> Result<Vec<u8>> {
    let secret = format!("AWS4{}", secret_key);
    let mut date_hmac = HmacSha256::new_from_slice(secret.as_bytes()).chain_err(|| "error hashing secret")?;
    date_hmac.update(date_time.format(SHORT_DATE_FMT).to_string().as_bytes());
    let mut region_hmac = HmacSha256::new_from_slice(&date_hmac.finalize().into_bytes()).chain_err(|| "error hashing date")?;
    region_hmac.update(region.to_string().as_bytes());
    let mut service_hmac = HmacSha256::new_from_slice(&region_hmac.finalize().into_bytes()).chain_err(|| "error hashing region")?;
    service_hmac.update(service.as_bytes());
    let mut signing_hmac = HmacSha256::new_from_slice(&service_hmac.finalize().into_bytes()).chain_err(|| "error hashing service")?;
    signing_hmac.update(b"aws4_request");
    Ok(signing_hmac.finalize().into_bytes().to_vec())
}

// -----------------------------------------------------------------------------
/// Generate the AWS authorization header.
pub fn authorization_header(
    access_key: &str,
    date_time: &DateTime<Utc>,
    region: &str,
    signed_headers: &str,
    signature: &str,
) -> String {
    format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{scope},\
            SignedHeaders={signed_headers},Signature={signature}",
        access_key = access_key,
        scope = scope_string(date_time, region),
        signed_headers = signed_headers,
        signature = signature
    )
}

// -----------------------------------------------------------------------------
pub fn sign(
    method: &str,
    payload_hash: &str,
    url_string: &str,
    headers: &HeadersMap,
    date_time: &DateTime<Utc>,
    secret: &str,
    region: &str,
    service: &str,
) -> Result<String> {
    let url = Url::parse(url_string).chain_err(|| "error parsing url")?;
    let canonical = canonical_request(method, &url, &headers, payload_hash);

    let string_to_sign = string_to_sign(&date_time, &"us-east-1", &canonical);

    let signing_key =
        signing_key(&date_time, secret, &region, service)?;
    let mut hmac = Hmac::<Sha256>::new_from_slice(&signing_key).chain_err(|| "error hashing signing key")?;
    hmac.update(string_to_sign.as_bytes());
    Ok(hex::encode(hmac.finalize().into_bytes()))
}
// -----------------------------------------------------------------------------
pub struct Signature {
    pub auth_header: String,
    pub date_time: String,
}

/// Return signed header and time-stamp.
pub fn signature(
    url: &url::Url,
    method: &str,
    access: &str,
    secret: &str,
    region: &str,
    service: &str,
    payload_hash: &str,
) -> Result<Signature> {
    const LONG_DATE_TIME: &str = "%Y%m%dT%H%M%SZ";
    let host_port = url.host().chain_err(|| "Error parsing host from url")?.to_string()
        + &if let Some(port) = url.port() {
            format!(":{}", port)
        } else {
            "".to_string()
        };
    let method = method.to_uppercase();
    let uri = url.as_str().trim_end_matches('/');
    let mut headers = HeadersMap::new();
    headers.insert("host".to_string(), host_port);
    headers.insert(
        "x-amz-content-sha256".to_string(),
        payload_hash.to_string(),
    );
    let date_time = Utc::now();
    let date_time_string = date_time.format(LONG_DATE_TIME).to_string();
    headers.insert("x-amz-date".to_string(), date_time_string.clone());
    let signature = sign(
        &method,
        payload_hash,
        &uri,
        &headers,
        &date_time,
        secret,
        region,
        service,
    )?;
    let auth = authorization_header(
        &access,
        &date_time,
        &region,
        &signed_header_string(&headers),
        &signature,
    );
    Ok(Signature {
        auth_header: auth,
        date_time: date_time_string,
    })
}

//------------------------------------------------------------------------------
/// Generate pre-signed URL
pub fn pre_signed_url(
    access: &str,
    secret: &str,
    expiration: u64,
    url: &Url,
    method: &str,
    payload_hash: &str,
    region: &str,
    date_time: &DateTime<Utc>,
    service: &str,
) -> Result<String> {
    let date_time_txt = date_time.format(LONG_DATETIME_FMT).to_string();
    let short_date_time_txt = date_time.format(SHORT_DATE_FMT).to_string();
    let credentials = format!(
        "{}/{}/{}/s3/aws4_request",
        access, short_date_time_txt, region
    );
    let mut params = BTreeMap::from([
        (
            "X-Amz-Algorithm".to_string(),
            "AWS4-HMAC-SHA256".to_string(),
        ),
        ("X-Amz-Credential".to_string(), credentials),
        ("X-Amz-Date".to_string(), date_time_txt),
        ("X-Amz-Expires".to_string(), expiration.to_string()),
        ("X-Amz-SignedHeaders".to_string(), "host".to_string()),
    ]);
    url.query_pairs().for_each(|(k, v)| {
        params.insert(k.to_string(), v.to_string());
    });
    let canonical_query_string = params
        .iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                url_encode(&k).to_owned(),
                url_encode(&v).to_owned()
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    let canonical_resource = url.path();
    let canonical_headers = "host:".to_owned()
        + &url
            .host()
            .ok_or("Error parsing host from url".to_owned())?
            .to_string();
    let signed_headers = "host";
    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n\n{}\n{}",
        method,
        canonical_resource,
        canonical_query_string,
        canonical_headers,
        signed_headers,
        payload_hash
    );
    let string_to_sign = string_to_sign(&date_time, &region, &canonical_request);
    let signing_key =
        signing_key(&date_time, secret, region, service)?;
    let mut hmac = Hmac::<Sha256>::new_from_slice(&signing_key).chain_err(|| "Error hashing signing key")?;
    hmac.update(string_to_sign.as_bytes());
    let signature = hex::encode(hmac.finalize().into_bytes());
    let request_url =
        url.to_string() + "?" + &canonical_query_string + "&X-Amz-Signature=" + &signature;

    Ok(request_url)
}

// Unit tests
//==============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeZone, Utc};

    #[test]
    fn test_signature() -> Result<()> {
        const EXPECTED_SIGNATURE: &str =
            "9c804edb9369936d72d48670640d9f2ea66581b2a02566355910ee23ba1dd59a";
        let url = "https://play.min.io/bucket/key";
        let method = "PUT";
        let payload_hash = "UNSIGNED-PAYLOAD";
        let date_time = Utc.ymd(2022, 2, 2).and_hms(0, 0, 0);
        let secret = "zuf+tfteSlswRu7BJ86wekitnifILbZam1KYY3TH";
        let region = "us-east-1";
        let service = "s3";
        let mut headers = HeadersMap::new();
        headers.insert("host".to_string(), "aws.com".to_string());
        headers.insert(
            "x-amz-content-sha256".to_string(),
            payload_hash.to_string(),
        );
        let signature = sign(
            method,
            payload_hash,
            url,
            &headers,
            &date_time,
            secret,
            region,
            service,
        )?;
        assert_eq!(EXPECTED_SIGNATURE, signature);
        Ok(())
    }

    #[test]
    fn test_presigned_url() -> Result<()> {
        const EXPECTED_URL: &str = "https://play.min.io/bucket/key?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=Q3AM3UQ867SPQQA43P2F%2F20220222%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20220222T202202Z&X-Amz-Expires=10000&X-Amz-SignedHeaders=host&X-Amz-Signature=add1518886b7a16b17fb88e335b664ea76edababa6bc9874b4af754a7aadb24a";

        let url = Url::parse("https://play.min.io/bucket/key").chain_err(|| "Error parsing url")?;
        let method = "GET";
        let payload_hash = "UNSIGNED-PAYLOAD";
        let access = "Q3AM3UQ867SPQQA43P2F";
        let secret = "zuf+tfteSlswRu7BJ86wekitnifILbZam1KYY3TG";
        let expiration = 10000_u64;
        let region = "us-east-1";
        let service = "s3";
        let dt = "2022-02-22T12:22:02-08:00";
        let date_time: DateTime<Utc> =
            DateTime::from(DateTime::parse_from_rfc3339(&dt).chain_err(|| "Error parsing date")?);
        let url = pre_signed_url(
            &access,
            &secret,
            expiration,
            &url,
            &method,
            &payload_hash,
            &region,
            &date_time,
            &service,
        )?;
        assert_eq!(EXPECTED_URL, url);
        Ok(())
    }
}