use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

pub struct GatedSse {
    pub url: String,
    pub deltas_sent: oneshot::Receiver<()>,
    pub complete: oneshot::Sender<()>,
    pub request: tokio::task::JoinHandle<std::io::Result<Vec<u8>>>,
}

impl GatedSse {
    pub async fn start(events: Vec<String>) -> std::io::Result<Self> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let (sent, deltas_sent) = oneshot::channel();
        let (complete, completion) = oneshot::channel();
        let request = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await?;
            let mut data = Vec::new();
            let body = loop {
                let mut buffer = [0; 4096];
                let count = socket.read(&mut buffer).await?;
                if count == 0 {
                    return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
                }
                data.extend_from_slice(&buffer[..count]);
                if let Some(end) = data.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&data[..end]);
                    let length = headers.lines().find_map(|line| {
                        let (key, value) = line.split_once(':')?;
                        key.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    });
                    let length = length.ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "missing request length",
                        )
                    })?;
                    if data.len() >= end + 4 + length {
                        break data[end + 4..end + 4 + length].to_vec();
                    }
                }
            };
            let (terminal, deltas) = events.split_last().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing terminal event")
            })?;
            let deltas = deltas.concat();
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                deltas.len() + terminal.len()
            );
            socket.write_all(header.as_bytes()).await?;
            socket.write_all(deltas.as_bytes()).await?;
            socket.flush().await?;
            let _ = sent.send(());
            completion.await.map_err(|error| {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, error.to_string())
            })?;
            socket.write_all(terminal.as_bytes()).await?;
            socket.shutdown().await?;
            Ok(body)
        });
        Ok(Self {
            url,
            deltas_sent,
            complete,
            request,
        })
    }
}
