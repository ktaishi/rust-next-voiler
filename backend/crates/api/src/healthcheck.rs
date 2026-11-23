//! コンテナのヘルスチェック用クライアント (`api healthcheck`)。
//!
//! distroless イメージには curl が無いため、バイナリ自身で自プロセスの `/healthz` を叩く。
//! ECS のコンテナヘルスチェック / docker compose の healthcheck / k8s の exec probe で共通に使える。

use std::time::Duration;

use anyhow::{bail, Context};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

/// `BIND_ADDR` (例 `0.0.0.0:8080`) からポートを取り出し、ループバックの接続先を作る。
pub fn local_target(bind_addr: &str) -> anyhow::Result<String> {
    let port = bind_addr
        .rsplit(':')
        .next()
        .and_then(|p| p.parse::<u16>().ok())
        .with_context(|| format!("invalid BIND_ADDR: {bind_addr}"))?;
    Ok(format!("127.0.0.1:{port}"))
}

/// `GET /healthz` を送り、200 なら Ok。
pub async fn check(target: &str, wait: Duration) -> anyhow::Result<()> {
    let status = timeout(wait, async {
        let mut s = TcpStream::connect(target).await?;
        s.write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await?;
        let mut buf = [0u8; 64];
        let n = s.read(&mut buf).await?;
        anyhow::Ok(String::from_utf8_lossy(&buf[..n]).into_owned())
    })
    .await
    .context("healthcheck timed out")??;

    match status.split_whitespace().nth(1) {
        Some("200") => Ok(()),
        other => bail!("unhealthy: status={other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_target_uses_bind_port() {
        assert_eq!(local_target("0.0.0.0:8080").unwrap(), "127.0.0.1:8080");
        assert_eq!(local_target("[::]:9000").unwrap(), "127.0.0.1:9000");
        assert!(local_target("nope").is_err());
    }
}
