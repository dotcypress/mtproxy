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
  pub secret: Vec<u8>,
  pub servers: HashMap<i16, Vec<SocketAddr>>,
}

impl Config {
  pub fn fetch(ipv6: bool) -> io::Result<Config> {
    let secret = get("/getProxySecret")?;
    let mut servers = HashMap::new();

    let path = if ipv6 {
      "/getProxyConfigV6"
    } else {
      "/getProxyConfig"
    };

    match get(path) {
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

    Ok(Config { secret, servers })
  }
}

fn get(path: &str) -> io::Result<Vec<u8>> {
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
