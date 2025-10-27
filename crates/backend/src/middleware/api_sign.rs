use base64ct::Encoding as _;
use hmac::Mac as _;
use sha2::Digest as _;
use sha2::Sha256;

type HmacSha512 = hmac::Hmac<sha2::Sha512>;

pub trait Nonce {
    fn nonce(&self) -> u64;
}

pub fn hmac_signature(uri_path: &str, nonce: &str, post_data: &str, secret: &[u8]) -> String {
    // Step 1: Concatenate nonce + POST data
    let nonce_postdata = format!("{}{}", nonce, post_data);

    // Step 2: SHA256 hash
    let mut sha256_hasher = Sha256::new();
    sha256_hasher.update(nonce_postdata.as_bytes());
    let sha256_digest = sha256_hasher.finalize();

    // Step 3: Concatenate URI path + SHA256 digest
    let mut message = uri_path.as_bytes().to_vec();
    message.extend_from_slice(&sha256_digest);

    // Step 4: HMAC-SHA512
    let mut mac = HmacSha512::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(&message);
    let hmac_result = mac.finalize();

    // Step 5: Base64 encode
    base64ct::Base64::encode_string(&hmac_result.into_bytes())
}

// // Check if Kraken HMAC signature verification is requested
// if let Some(api_sign) = request
//     .headers()
//     .get("API-Sign")
//     .and_then(|v| v.to_str().ok())
//     .map(|s| s.to_owned())
// {
//     let uri_path = request.uri().path().to_owned();
//     let (parts, body) = request.into_parts();
//     let body_bytes = match body.collect().await {
//         Ok(collected) => collected.to_bytes(),
//         Err(_) => {
//             return (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "Failed to read request body",
//             )
//                 .into_response();
//         }
//     };
//     let body_str = String::from_utf8_lossy(&body_bytes);
//     let Some(nonce) = extract_nonce_from_body(&body_str) else {
//         return (StatusCode::BAD_REQUEST, "Missing nonce in request body").into_response();
//     };
//     let computed_signature = hmac_signature(&uri_path, &nonce, &body_str, todo!());
//     if computed_signature != api_sign {
//         return (StatusCode::UNAUTHORIZED, "Invalid HMAC signature").into_response();
//     }
//     request = Request::from_parts(parts, Body::from(body_bytes));
// }
