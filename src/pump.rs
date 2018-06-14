use mio::{net::TcpStream, unix::UnixReady, Ready};
use proto::Proto;
use std::io::{self, prelude::*, ErrorKind};
use std::{mem, u16};

const BUF_SIZE: usize = 128 * 1024;
const MAX_READ_BUF_SIZE: usize = u16::MAX as usize * 2;

pub struct Pump {
  sock: TcpStream,
  secret: Vec<u8>,
  proto: Option<Proto>,
  read_buf: Vec<u8>,
  write_buf: Vec<u8>,
  interest: Ready,
}

impl Pump {
  pub fn upstream(secret: &[u8], sock: TcpStream) -> Pump {
    Pump {
      sock,
      secret: secret.to_vec(),
      proto: None,
      read_buf: Vec::with_capacity(BUF_SIZE),
      write_buf: Vec::with_capacity(BUF_SIZE),
      interest: Ready::readable() | UnixReady::error() | UnixReady::hup(),
    }
  }

  pub fn downstream(secret: &[u8], sock: TcpStream) -> Pump {
    let proto = Proto::new(secret);
    let mut write_buf = Vec::with_capacity(BUF_SIZE);
    write_buf.append(&mut proto.seed().to_vec());

    Pump {
      sock,
      secret: secret.to_vec(),
      proto: Some(proto),
      interest: Ready::readable() | Ready::writable() | UnixReady::error() | UnixReady::hup(),
      read_buf: Vec::with_capacity(BUF_SIZE),
      write_buf,
    }
  }

  pub fn sock(&self) -> &TcpStream {
    &self.sock
  }

  pub fn interest(&self) -> Ready {
    self.interest
  }

  pub fn push(&mut self, input: &[u8]) {
    match self.proto {
      Some(ref mut proto) => {
        let mut buf = vec![0u8; input.len()];
        proto.enc(input, &mut buf);
        self.write_buf.append(&mut buf);
        self.interest.insert(Ready::writable());
      }
      None => {
        debug!("failed to push. protocol not ready.");
      }
    }
  }

  pub fn pull(&mut self) -> Vec<u8> {
    if self.read_buf.is_empty() {
      return vec![];
    }
    match self.proto {
      Some(ref mut proto) => {
        let mut buf = vec![0u8; self.read_buf.len()];
        proto.dec(&self.read_buf, &mut buf);
        self.read_buf.clear();
        self.interest.insert(Ready::readable());
        buf
      }
      None => {
        debug!("failed to pull. protocol not ready.");
        vec![]
      }
    }
  }

  pub fn flush(&mut self) -> io::Result<()> {
    loop {
      match self.sock.write(&self.write_buf) {
        Ok(n) => {
          trace!("write {} bytes", n);
          let mut rest = self.write_buf.split_off(n);
          mem::swap(&mut rest, &mut self.write_buf);
          if self.write_buf.is_empty() {
            self.interest.remove(Ready::writable());
            break;
          }
        }
        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
          break;
        }
        Err(e) => return Err(e),
      }
    }
    Ok(())
  }

  pub fn drain(&mut self) -> io::Result<Option<i16>> {
    let mut link_pending = None;

    loop {
      if self.read_buf.len() > MAX_READ_BUF_SIZE {
        debug!("read buffer is full");
        self.interest.remove(Ready::readable());
        break;
      }
      let mut buf = vec![0u8; BUF_SIZE];
      match self.sock.read(&mut buf) {
        Ok(0) => break,
        Ok(n) => {
          trace!("read {} bytes", n);
          buf.split_off(n);
          self.read_buf.extend(buf);

          if self.proto.is_none() {
            if self.read_buf.len() == 41 {
              return Err(io::Error::new(io::ErrorKind::Other, "Fake PQ req"));
            }
            if self.read_buf.len() >= 64 {
              let mut seed = self.read_buf.split_off(64);
              mem::swap(&mut seed, &mut self.read_buf);
              let proto = Proto::from_seed(&seed, &self.secret)?;
              link_pending = Some(proto.dc());
              self.proto = Some(proto);
            }
          }
        }
        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
          break;
        }
        Err(e) => return Err(e),
      }
    }
    Ok(link_pending)
  }
}
