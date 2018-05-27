use mio::net::TcpStream;
use std::collections::VecDeque;
use std::io;
use std::net::SocketAddr;

lazy_static! {
  static ref DATA_CENTERS: [SocketAddr; 5] = {
    [
      "149.154.175.50:443".parse().unwrap(),
      "149.154.167.51:443".parse().unwrap(),
      "149.154.175.100:443".parse().unwrap(),
      "149.154.167.91:443".parse().unwrap(),
      "149.154.171.5:443".parse().unwrap(),
    ]
  };
}

const POOL_SIZE: usize = 16;

pub struct DcPool {
  conns: Vec<VecDeque<TcpStream>>,
}

impl DcPool {
  pub fn new() -> DcPool {
    let mut pool = DcPool {
      conns: vec![
        VecDeque::with_capacity(POOL_SIZE),
        VecDeque::with_capacity(POOL_SIZE),
        VecDeque::with_capacity(POOL_SIZE),
        VecDeque::with_capacity(POOL_SIZE),
        VecDeque::with_capacity(POOL_SIZE),
      ],
    };
    pool.invalidate();
    pool
  }

  pub fn invalidate(&mut self) {
    for (dc_idx, dc_conns) in self.conns.iter_mut().enumerate() {
      while dc_conns.len() < POOL_SIZE {
        info!(
          "connecting to dc: {:?} @ {:?}",
          dc_idx, &DATA_CENTERS[dc_idx]
        );
        let stream = TcpStream::connect(&DATA_CENTERS[dc_idx]).expect("DC fail");
        dc_conns.push_front(stream);
      }
    }
  }

  pub fn get(&mut self, dc_idx: usize) -> io::Result<TcpStream> {
    let queue = &mut self.conns[dc_idx];
    match queue.pop_back() {
      Some(stream) => Ok(stream),
      None => {
        warn!("dc pool is empty: {}", dc_idx);
        TcpStream::connect(&DATA_CENTERS[dc_idx])
      }
    }
  }
}
