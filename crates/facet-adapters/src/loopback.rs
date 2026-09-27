//! [`LoopbackListener`] on the standard library's TCP listener, bound to `127.0.0.1` on a port the system
//! assigns.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use facet_core::port::{LoopbackListener, LoopbackSession};

pub struct StdLoopbackListener;

impl LoopbackListener for StdLoopbackListener {
    fn bind(&self) -> Result<Box<dyn LoopbackSession>, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
        listener.set_nonblocking(true).map_err(|error| error.to_string())?;
        let port = listener.local_addr().map_err(|error| error.to_string())?.port();
        Ok(Box::new(StdSession { listener, port }))
    }
}

struct StdSession {
    listener: TcpListener,
    port: u16,
}

impl LoopbackSession for StdSession {
    fn port(&self) -> u16 {
        self.port
    }

    fn next(
        &mut self,
        timeout: Duration,
        respond: &dyn Fn(&str) -> String,
    ) -> Result<Option<String>, String> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => return answer(stream, respond).map(Some),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Ok(None);
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(error) => return Err(error.to_string()),
            }
        }
    }
}

/// Reads the request line from `stream`, writes `respond`'s reply to it, and returns the line.
fn answer(stream: TcpStream, respond: &dyn Fn(&str) -> String) -> Result<String, String> {
    stream.set_nonblocking(false).map_err(|error| error.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|error| error.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|error| format!("the request could not be read: {error}"))?;
    let line = line.trim_end().to_string();
    let mut stream = reader.into_inner();
    stream
        .write_all(respond(&line).as_bytes())
        .map_err(|error| format!("the reply could not be sent: {error}"))?;
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_is_answered_and_its_line_returned() {
        let mut session = StdLoopbackListener.bind().expect("a port should bind");
        let port = session.port();
        assert_ne!(port, 0);
        let client = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("should connect");
            stream.write_all(b"GET /?state=s&code=c HTTP/1.1\r\nHost: x\r\n\r\n").expect("should send");
            let mut reply = String::new();
            std::io::Read::read_to_string(&mut stream, &mut reply).expect("should read");
            reply
        });
        let line = session
            .next(Duration::from_secs(5), &|_| "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok".to_string());
        assert_eq!(line, Ok(Some("GET /?state=s&code=c HTTP/1.1".to_string())));
        assert!(client.join().expect("client").ends_with("ok"));
        assert_eq!(session.next(Duration::from_millis(200), &|_| String::new()), Ok(None));
    }
}
