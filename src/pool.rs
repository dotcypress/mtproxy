use config::Config;
use mio::net::TcpStream;
use pump::Pump;
use rand;
use rand::Rng;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MIN_POOL_SIZE: usize = 16;

pub struct DcPool {
  secret: Vec<u8>,
  ipv6: bool,
  conns: Arc<Mutex<HashMap<i16, VecDeque<TcpStream>>>>,
}

impl DcPool {
  pub fn new(ipv6: bool) -> DcPool {
    DcPool {
      ipv6,
      secret: vec![],
      conns: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  pub fn start(&mut self) -> io::Result<()> {
    let config = Config::fetch(self.ipv6)?;

    debug!("Starting connection pool. IPv6: {:?}", self.ipv6);
    let conns = self.conns.clone();
    thread::spawn(move || {
      let mut rng = rand::thread_rng();
      loop {
        for dc in config.servers.keys() {
          let mut conns = conns.lock().unwrap();
          let dc_conns = conns.entry(*dc).or_insert_with(|| VecDeque::new());
          while dc_conns.len() < MIN_POOL_SIZE {
            let addr = rng.choose(&config.servers.get(dc).unwrap()).unwrap();
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
