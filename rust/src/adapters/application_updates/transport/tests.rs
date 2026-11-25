use super::*;

#[test]
fn redirects_stay_on_release_hosts_and_https() {
    for url in [
        "http://github.com/a",
        "https://github.com.example.com/a",
        "https://github.com:444/a",
        "https://user@github.com/a",
        "https://raw.githubusercontent.com/a",
    ] {
        assert!(!admitted_url(&Url::parse(url).unwrap()), "{url}");
    }
    assert!(admitted_url(
        &Url::parse("https://release-assets.githubusercontent.com/a?token=test").unwrap()
    ));
}

#[test]
fn resumed_responses_cannot_silently_append_the_wrong_range() {
    let mut headers = header::HeaderMap::new();
    headers.insert(header::CONTENT_RANGE, "bytes 100-999/1000".parse().unwrap());
    headers.insert(header::CONTENT_LENGTH, "900".parse().unwrap());
    assert_eq!(
        response_offset(StatusCode::PARTIAL_CONTENT, &headers, 100, 1000).unwrap(),
        100
    );
    assert!(response_offset(StatusCode::PARTIAL_CONTENT, &headers, 200, 1000).is_err());
    headers.insert(header::CONTENT_RANGE, "bytes 100-999/2000".parse().unwrap());
    assert!(response_offset(StatusCode::PARTIAL_CONTENT, &headers, 100, 1000).is_err());
    headers.remove(header::CONTENT_RANGE);
    headers.insert(header::CONTENT_LENGTH, "1000".parse().unwrap());
    assert_eq!(
        response_offset(StatusCode::OK, &headers, 100, 1000).unwrap(),
        0
    );
    headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
    assert!(response_offset(StatusCode::OK, &headers, 100, 1000).is_err());
}
