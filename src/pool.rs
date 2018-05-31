use std::collections::{HashMap, VecDeque};
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use mio::net::TcpStream;
use pump::Pump;
use rand;
use rand::Rng;
use reqwest;

const MIN_POOL_SIZE: usize = 8;

pub struct DcPool {
  secret: Vec<u8>,
  conns: Arc<Mutex<HashMap<i16, VecDeque<TcpStream>>>>,
}

impl DcPool {
  pub fn new() -> DcPool {
    DcPool {
      secret: vec![],
      conns: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  pub fn start(&mut self) -> io::Result<()> {
    let config = load_config()?;
    self.secret = load_secret()?;

    let conns = self.conns.clone();
    thread::spawn(move || {
      let mut rng = rand::thread_rng();
      loop {
        for dc in config.keys() {
          let mut conns = conns.lock().unwrap();
          let dc_conns = conns.entry(*dc).or_insert_with(|| VecDeque::new());
          while dc_conns.len() < MIN_POOL_SIZE {
            let addr = rng.choose(&config.get(dc).unwrap()).unwrap();
            match TcpStream::connect(addr) {
              Ok(stream) => dc_conns.push_front(stream),
              Err(err) => error!("DC connection failed: #{}", err),
            }
          }
        }
        thread::sleep(Duration::from_millis(100));
      }
    });
    Ok(())
  }

  pub fn get(&mut self, dc: i16) -> Option<Pump> {
    match self.conns.lock().unwrap().get_mut(&dc) {
      None => None,
      Some(queue) => match queue.pop_back() {
        Some(stream) => Some(Pump::upstream(&self.secret, stream)),
        None => {
          error!("dc connection pool is empty: #{}", dc);
          None
        }
      },
    }
  }
}

fn load_secret() -> io::Result<Vec<u8>> {
  match reqwest::get("https://core.telegram.org/getProxySecret") {
    Ok(mut resp) => {
      let mut buf: Vec<u8> = vec![];
      match resp.copy_to(&mut buf) {
        Err(err) => {
          error!("Failed to load proxy secret: {:?}", err);
          return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to load proxy secret",
          ));
        }
        Ok(_) => {}
      }
      return Ok(buf);
    }
    Err(err) => {
      error!("Failed to load proxy secret: {:?}", err);
      return Err(io::Error::new(
        io::ErrorKind::Other,
        "Failed to load proxy secret",
      ));
    }
  };
}

fn load_config() -> io::Result<HashMap<i16, Vec<SocketAddr>>> {
  let mut config = HashMap::new();
  match reqwest::get("https://core.telegram.org/getProxyConfig") {
    Ok(mut resp) => match resp.text() {
      Ok(text) => {
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

          let dc_config = config.entry(dc).or_insert_with(|| vec![]);
          dc_config.push(addr);
        }
      }
      Err(err) => {
        error!("Failed to parse proxy config: {:?}", err);
        return Err(io::Error::new(
          io::ErrorKind::Other,
          "Failed to parse proxy config",
        ));
      }
    },
    Err(err) => {
      error!("Failed to load proxy config: {:?}", err);
      return Err(io::Error::new(
        io::ErrorKind::Other,
        "Failed to load proxy config",
      ));
    }
  };
  Ok(config)
}
