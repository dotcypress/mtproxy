use std::io::{self, prelude::*};
use std::{mem, u16};

use mio::{net::TcpStream, unix::UnixReady, Ready};

use proto::Proto;

const BUF_SIZE: usize = u16::MAX as usize;
const MAX_READ_BUF_SIZE: usize = BUF_SIZE * 512;

pub struct Pump {
  sock: TcpStream,
  secret: Vec<u8>,
  proto: Proto,
  read_buf: Vec<u8>,
  write_buf: Vec<u8>,
  interest: Ready,
}

impl Pump {
  pub fn new(sock: TcpStream, secret: &[u8]) -> Pump {
    Pump {
      sock,
      secret: secret.to_vec(),
      proto: Proto::default(),
      read_buf: Vec::with_capacity(BUF_SIZE),
      write_buf: Vec::with_capacity(BUF_SIZE),
      interest: Ready::readable() | UnixReady::error() | UnixReady::hup(),
    }
  }

  pub fn sock(&self) -> &TcpStream {
    &self.sock
  }

  pub fn ready(&self) -> bool {
    !self.proto.seed().is_empty()
  }

  pub fn interest(&self) -> Ready {
    self.interest
  }

  pub fn push(&mut self, input: &[u8]) {
    if !self.ready() {
      debug!("failed to push. protocol not ready.");
      return;
    }
    let mut buf = vec![0u8; input.len()];
    self.proto.enc(input, &mut buf);
    self.write_buf.append(&mut buf);
    if !self.write_buf.is_empty() {
      self.interest.insert(Ready::writable());
    }
  }

  pub fn pull(&mut self) -> Vec<u8> {
    if !self.ready() {
      debug!("failed to pull. protocol not ready.");
      return vec![];
    }

    let mut buf = vec![0u8; self.read_buf.len()];
    self.proto.dec(&self.read_buf, &mut buf);
    self.read_buf.clear();
    self.interest.insert(Ready::readable());
    buf
  }

  pub fn flush(&mut self) -> io::Result<()> {
    match self.sock.write(&self.write_buf) {
      Ok(0) => {
        trace!("flush zero");
      }
      Ok(n) => {
        trace!("write {} bytes", n);
        let mut rest = self.write_buf.split_off(n);
        mem::swap(&mut rest, &mut self.write_buf);
      }
      Err(e) => return Err(e),
    }
    if self.write_buf.is_empty() {
      self.interest.remove(Ready::writable());
      return Err(io::Error::from(io::ErrorKind::WouldBlock));
    }
    Ok(())
  }

  pub fn drain(&mut self) -> io::Result<Option<Pump>> {
    if self.read_buf.len() > MAX_READ_BUF_SIZE {
      debug!("read buffer is full");
      self.interest.remove(Ready::readable());
      return Err(io::Error::from(io::ErrorKind::WouldBlock));
    }
    let mut buf = vec![0u8; BUF_SIZE];
    match self.sock.read(&mut buf) {
      Ok(0) => return Err(io::Error::from(io::ErrorKind::WouldBlock)),
      Ok(n) => {
        trace!("read {} bytes", n);
        buf.split_off(n);
        self.read_buf.extend(buf);

        if !self.ready() && self.read_buf.len() >= 64 {
          let mut seed = self.read_buf.split_off(64);
          mem::swap(&mut seed, &mut self.read_buf);
          self.proto = Proto::from_seed(&seed, &self.secret)?;
          trace!("connected to Tg server @ {}", self.proto.dc());
          let sock = TcpStream::connect(self.proto.dc())?;
          let proto = Proto::new();

          let mut write_buf = Vec::with_capacity(BUF_SIZE);
          write_buf.append(&mut proto.seed().to_vec());

          return Ok(Some(Pump {
            sock,
            secret: vec![],
            proto,
            interest: Ready::readable() | Ready::writable() | UnixReady::error() | UnixReady::hup(),
            read_buf: Vec::with_capacity(BUF_SIZE),
            write_buf,
          }));
        }
        return Ok(None);
      }
      Err(e) => return Err(e),
    }
  }
}
