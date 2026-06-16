use std::net::SocketAddr;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct MockServerHandle {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
}

impl MockServerHandle {
    pub fn new(addr: SocketAddr, shutdown: oneshot::Sender<()>) -> Self {
        Self {
            addr,
            shutdown: Some(shutdown),
        }
    }

    pub fn endpoint(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl Drop for MockServerHandle {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}
