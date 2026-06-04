//! TCP tunnel to the upstream backend.

use crate::common::{Error, Result};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// A trivial TCP forwarding proxy. Used for CONNECT-style requests and raw
/// WebSocket passthrough.
#[derive(Debug)]
pub struct TcpUpstream;

impl TcpUpstream {
    /// Accept connections on `bind_addr` and forward each one to
    /// `target_addr`.
    pub async fn serve(bind_addr: SocketAddr, target_addr: SocketAddr) -> Result<()> {
        let listener = TcpListener::bind(bind_addr).await?;
        loop {
            let (client, _peer) = listener.accept().await?;
            let target = target_addr;
            tokio::spawn(async move {
                if let Err(e) = pump(client, target).await {
                    tracing::debug!("tcp upstream pump ended: {e}");
                }
            });
        }
    }
}

async fn pump(client: TcpStream, target: SocketAddr) -> Result<()> {
    let upstream = TcpStream::connect(target)
        .await
        .map_err(|e| Error::network(format!("upstream connect: {e}")))?;
    // Split into owned halves using `into_split` (TcpStream -> ReadHalf/WriteHalf
    // that are 'static).
    let (mut cr, mut cw) = client.into_split();
    let (mut ur, mut uw) = upstream.into_split();
    let c2u = tokio::spawn(async move {
        let mut buf = vec![0u8; 8192];
        loop {
            match cr.read(&mut buf).await {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    if uw.write_all(&buf[..n]).await.is_err() {
                        return;
                    }
                }
            }
        }
    });
    let u2c = tokio::spawn(async move {
        let mut buf = vec![0u8; 8192];
        loop {
            match ur.read(&mut buf).await {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    if cw.write_all(&buf[..n]).await.is_err() {
                        return;
                    }
                }
            }
        }
    });
    let _ = tokio::join!(c2u, u2c);
    Ok(())
}
