use rand;
use rand::Rng;
use rustls::{ClientConfig, ClientSession, Stream};
use std::collections::HashMap;
use std::io;
use std::io::Read;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use webpki::DNSNameRef;
use webpki_roots::TLS_SERVER_ROOTS;

pub struct Config {
  bind_addr: SocketAddr,
  secret: Vec<u8>,
  dc_secret: Vec<u8>,
  servers: HashMap<i16, Vec<SocketAddr>>,
}

impl Config {
  pub fn init(
    bind_addr: SocketAddr,
    secret: Vec<u8>,
    _tag: Option<Vec<u8>>,
    ipv6: bool,
  ) -> io::Result<Config> {
    let dc_secret = Config::http_get("/getProxySecret")?;
    let mut servers = HashMap::new();

    let path = if ipv6 {
      "/getProxyConfigV6"
    } else {
      "/getProxyConfig"
    };

    match Config::http_get(path) {
      Ok(buf) => {
        let text = String::from_utf8_lossy(&buf);
        for line in text.lines() {
          if !line.starts_with("proxy_for") {
            continue;
          }

          let chunks: Vec<&str> = line.splitn(3, " ").collect();
          let dc: i16 = chunks[1].parse().or(Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to parse proxy config",
          )))?;

          let mut addr = String::from(chunks[2]);
          addr.pop();
          let addr = addr.parse().or(Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to parse proxy config",
          )))?;

          let dc_config = servers.entry(dc).or_insert_with(|| vec![]);
          dc_config.push(addr);
        }
      }
      Err(err) => {
        error!("Failed to load proxy config: {:?}", err);
        return Err(io::Error::new(
          io::ErrorKind::Other,
          "Failed to load proxy config",
        ));
      }
    };

    Ok(Config {
      secret,
      dc_secret,
      servers,
      bind_addr,
    })
  }

  pub fn bind_addr(&self) -> &SocketAddr {
    &self.bind_addr
  }

  pub fn secret(&self) -> &[u8] {
    &self.secret
  }

  pub fn dc_addr(&self, dc_idx: i16) -> Option<&SocketAddr> {
    let mut rng = rand::thread_rng();
    self
      .servers
      .get(&dc_idx)
      .and_then(|servers| rng.choose(&servers))
  }

  pub fn dc_secret(&self) -> &[u8] {
    &self.dc_secret
  }

  fn http_get(path: &str) -> io::Result<Vec<u8>> {
    let mut config = ClientConfig::new();
    config
      .root_store
      .add_server_trust_anchors(&TLS_SERVER_ROOTS);

    let dns_name = DNSNameRef::try_from_ascii_str("core.telegram.org").unwrap();
    let mut sess = ClientSession::new(&Arc::new(config), dns_name);
    let mut sock = TcpStream::connect("core.telegram.org:443")?;
    let mut tls = Stream::new(&mut sess, &mut sock);
    let payload = format!(
    "GET {} HTTP/1.1\r\nHost: core.telegram.org\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n",
    path
  );
    tls.write_all(payload.as_bytes())?;
    let mut buf = Vec::new();
    tls.read_to_end(&mut buf).unwrap_or(0);
    Ok(buf)
  }
}
