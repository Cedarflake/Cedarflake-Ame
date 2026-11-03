use std::io::{Read, Write};
use std::net::TcpListener;

use super::*;

#[test]
fn redirect_policy_rejects_plaintext_credentials_ports_and_unrelated_hosts() {
    for url in [
        "http://huggingface.co/file",
        "https://huggingface.co.evil.example/file",
        "https://evil.example/file",
        "https://huggingface.co:444/file",
        "https://user@huggingface.co/file",
        "https://127.0.0.1/file",
    ] {
        assert!(!admitted_url(&Url::parse(url).unwrap()), "{url}");
    }
    for url in [
        "https://huggingface.co/file",
        "https://cas-bridge.xethub.hf.co/file",
        "https://cdn-lfs.huggingface.co/file",
    ] {
        assert!(admitted_url(&Url::parse(url).unwrap()));
    }
}

fn response(headers: &'static str) -> Response {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 2048];
        assert!(stream.read(&mut request).unwrap() > 0);
        write!(stream, "{headers}\r\nConnection: close\r\n\r\nabc").unwrap();
    });
    let response = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap()
        .get(format!("http://{address}/fixture"))
        .send()
        .unwrap();
    server.join().unwrap();
    response
}

#[test]
fn server_status_length_and_encoding_are_validated_before_streaming() {
    let artifact = Artifact {
        name: "fixture",
        length: 3,
        digest: "unused",
    };
    for headers in [
        "HTTP/1.1 404 Not Found\r\nContent-Length: 3",
        "HTTP/1.1 206 Partial Content\r\nContent-Length: 3",
        "HTTP/1.1 200 OK\r\nContent-Length: 2",
        "HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 3",
    ] {
        assert!(admitted_response(&response(headers), &artifact, 0).is_err());
    }
    let mut valid = response("HTTP/1.1 200 OK\r\nContent-Length: 3");
    admitted_response(&valid, &artifact, 0).unwrap();
    let mut body = String::new();
    valid.read_to_string(&mut body).unwrap();
    assert_eq!(body, "abc");
}

#[test]
fn stalled_body_is_bounded_and_its_partial_file_can_resume() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (finish, finished) = std::sync::mpsc::channel::<()>();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 2048];
        assert!(stream.read(&mut request).unwrap() > 0);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\na")
            .unwrap();
        let _ = finished.recv_timeout(Duration::from_secs(5));
    });
    let response = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap()
        .get(format!("http://{address}/fixture"))
        .send()
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let artifact = Artifact {
        name: "fixture",
        length: 3,
        digest: "unused",
    };
    struct OneResponse(Option<Response>);
    impl ModelDownload for OneResponse {
        fn open(&mut self, _: &Artifact, offset: u64) -> Result<DownloadBody, ScanError> {
            Ok(DownloadBody {
                reader: Box::new(self.0.take().unwrap()),
                offset,
            })
        }
    }
    let result = super::super::transfer::download_artifact(
        directory.path(),
        &artifact,
        &mut OneResponse(Some(response)),
        &std::sync::atomic::AtomicBool::new(false),
        &mut |_| {},
    );
    drop(finish);
    server.join().unwrap();
    assert_eq!(
        result.unwrap_err().code,
        "classification_download_interrupted"
    );
    let entries = std::fs::read_dir(directory.path())
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(std::fs::read(entries[0].path()).unwrap(), b"a");
}

#[test]
fn only_the_exact_requested_range_or_a_complete_response_is_admitted() {
    let artifact = Artifact {
        name: "fixture",
        length: 3,
        digest: "unused",
    };
    for headers in [
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-2/3\r\nContent-Length: 3",
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 1-2/4\r\nContent-Length: 2",
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 1-1/3\r\nContent-Length: 1",
        "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 3",
    ] {
        assert!(admitted_response(&response(headers), &artifact, 1).is_err());
    }
    assert_eq!(
        admitted_response(
            &response(
                "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 1-2/3\r\nContent-Length: 2"
            ),
            &artifact,
            1
        )
        .unwrap(),
        1
    );
    assert_eq!(
        admitted_response(
            &response("HTTP/1.1 200 OK\r\nContent-Length: 3"),
            &artifact,
            1
        )
        .unwrap(),
        0
    );
}
