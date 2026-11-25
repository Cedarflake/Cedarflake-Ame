use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub enum Reply {
    Full,
    Range,
    WrongRange,
    Interrupt(usize),
}

pub struct Server {
    pub address: SocketAddr,
    task: JoinHandle<Vec<Option<u64>>>,
}

impl Server {
    pub fn start(body: Vec<u8>, replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let task = thread::spawn(move || {
            let mut ranges = Vec::new();
            for reply in replies {
                let deadline = Instant::now() + Duration::from_secs(5);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(
                                Instant::now() < deadline,
                                "Update fixture received no request"
                            );
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("Update fixture accept failed: {error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    assert!(request.len() < 4096);
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                }
                let request = String::from_utf8(request).unwrap().to_ascii_lowercase();
                let range = request.lines().find_map(|line| {
                    line.strip_prefix("range: bytes=")?
                        .strip_suffix('-')?
                        .parse::<u64>()
                        .ok()
                });
                ranges.push(range);
                let (offset, status, content_range, count) = match reply {
                    Reply::Full => (0, "200 OK", String::new(), body.len()),
                    Reply::Interrupt(count) => (0, "200 OK", String::new(), count),
                    Reply::Range | Reply::WrongRange => {
                        let offset = range.unwrap() as usize;
                        let advertised = if matches!(reply, Reply::WrongRange) {
                            offset + 1
                        } else {
                            offset
                        };
                        (
                            offset,
                            "206 Partial Content",
                            format!(
                                "Content-Range: bytes {advertised}-{}/{}\r\n",
                                body.len() - 1,
                                body.len(),
                            ),
                            body.len() - offset,
                        )
                    }
                };
                let header = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{content_range}Connection: close\r\n\r\n",
                    body.len() - offset
                );
                stream.write_all(header.as_bytes()).unwrap();
                // Cancellation may close the connection while the fixture is writing its body.
                let _ = stream.write_all(&body[offset..offset + count]);
            }
            ranges
        });
        Self { address, task }
    }

    pub fn join(self) -> Vec<Option<u64>> {
        self.task.join().unwrap()
    }
}
