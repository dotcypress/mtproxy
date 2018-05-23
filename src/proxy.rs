use std::{
  cell::RefCell, collections::{HashMap, HashSet}, io::{self, ErrorKind}, net::SocketAddr, thread,
  time::Duration, usize,
};

use crypto::{digest::Digest, sha2::Sha256};
use mio::{net::TcpListener, unix::UnixReady, Events, Poll, PollOpt, Ready, Token};
use pump::Pump;
use slab::Slab;

const MAX_PUMPS: usize = 2048;
const ROOT_TOKEN: Token = Token(<usize>::max_value() - 1);

pub struct Server {
  sock: TcpListener,
  poll: Poll,
  secret: Vec<u8>,
  pumps: Slab<RefCell<Pump>>,
  links: HashMap<Token, Token>,
}

impl Server {
  pub fn new(addr: SocketAddr, seed: &str) -> Server {
    let mut sha = Sha256::new();
    let mut secret = vec![0u8; sha.output_bytes()];

    sha.input_str(seed);
    sha.result(&mut secret);
    secret.truncate(16);

    Server {
      secret,
      sock: TcpListener::bind(&addr).expect("Failed to bind"),
      poll: Poll::new().expect("Failed to create Poll"),
      pumps: Slab::with_capacity(MAX_PUMPS),
      links: HashMap::new(),
    }
  }

  pub fn secret(&self) -> String {
    let secret: Vec<String> = self.secret.iter().map(|b| format!("{:02x}", b)).collect();
    secret.join("")
  }

  pub fn run(&mut self) -> io::Result<()> {
    info!("Starting proxy");
    self
      .poll
      .register(&self.sock, ROOT_TOKEN, Ready::readable(), PollOpt::edge())?;

    let mut events = Events::with_capacity(1024);

    loop {
      if self.poll.poll(&mut events, None)? == 0 {
        info!("idle");
        thread::sleep(Duration::from_millis(100));
      }

      let seen_tokens = self.dispatch(&events)?;

      for token in &seen_tokens {
        let pump = self.pumps.get(token.0);
        if pump.is_none() {
          continue;
        }
        let mut pump = pump.unwrap().borrow_mut();
        match self.links.get(token) {
          Some(peer_token) => {
            let buf = pump.pull();
            if !buf.is_empty() {
              let dst = self.pumps.get(peer_token.0).unwrap();
              let mut dst = dst.borrow_mut();
              dst.push(&buf);
            }
          }
          _ => {}
        }

        self.poll.reregister(
          pump.sock(),
          *token,
          Ready::readable() | Ready::writable() | UnixReady::hup(),
          PollOpt::edge() | PollOpt::oneshot(),
        )?;
      }
    }
  }

  fn accept(&mut self) -> io::Result<()> {
    if self.pumps.len() > MAX_PUMPS {
      warn!("max connection limit({}) exceeded", MAX_PUMPS / 2);
      return Ok(());
    }

    let sock = match self.sock.accept() {
      Ok((sock, _)) => sock,
      Err(err) => {
        warn!("accept failed: {}", err);
        return Ok(());
      }
    };

    let pump = Pump::new(sock, &self.secret);
    let idx = self.pumps.insert(RefCell::new(pump));
    let pump = self.pumps.get(idx).unwrap().borrow();

    let token = Token(idx);

    self.poll.register(
      pump.sock(),
      token,
      Ready::readable() | Ready::writable() | UnixReady::hup(),
      PollOpt::edge() | PollOpt::oneshot(),
    )?;

    info!(
      "new connection: {:?} from {}",
      token,
      pump.sock().peer_addr()?
    );

    Ok(())
  }

  fn dispatch(&mut self, events: &Events) -> io::Result<Vec<Token>> {
    let mut stale = HashSet::new();
    let mut completed = HashSet::new();
    let mut new_pumps = HashMap::new();

    for event in events {
      let token = event.token();

      if token == ROOT_TOKEN {
        self.accept()?;
        continue;
      }

      let readiness = UnixReady::from(event.readiness());
      if readiness.is_hup() {
        stale.insert(token);
        continue;
      }

      let mut pump = {
        let pump = &self.pumps.get(token.0);
        if pump.is_none() {
          warn!("slab inconsistency");
          continue;
        }
        pump.unwrap().borrow_mut()
      };

      if readiness.is_readable() {
        loop {
          match pump.drain() {
            Ok(ret) => match ret {
              Some(peer_pump) => {
                new_pumps.insert(token, peer_pump);
              }
              _ => {}
            },
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
              break;
            }
            Err(e) => {
              warn!("drain failed: {:?}: {}", token, e);
              stale.insert(token);
              break;
            }
          }
        }
      }

      if readiness.is_writable() {
        loop {
          match pump.flush() {
            Ok(_) => {}
            Err(ref e) if e.kind() == ErrorKind::WriteZero => {
              break;
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
              break;
            }
            Err(e) => {
              warn!("flush failed: {:?}: {}", token, e);
              stale.insert(token);
              break;
            }
          }
        }
      }

      completed.insert(token);
    }

    for token in stale {
      self.drop_pump(token);
    }

    for (owner, pump) in new_pumps {
      let idx = self.pumps.insert(RefCell::new(pump));
      let pump = self.pumps.get(idx).unwrap().borrow();

      let token = Token(idx);

      self.links.insert(token, owner);
      self.links.insert(owner, token);

      self.poll.register(
        pump.sock(),
        token,
        Ready::readable() | Ready::writable() | UnixReady::hup(),
        PollOpt::edge() | PollOpt::oneshot(),
      )?;
    }

    Ok(completed.into_iter().collect::<Vec<Token>>())
  }

  fn drop_pump(&mut self, token: Token) {
    trace!("dropping pump: {:?}", token);
    self.pumps.remove(token.0);
    match &self.links.remove(&token) {
      Some(peer_token) => {
        trace!("dropping pump peer: {:?}", peer_token);
        self.pumps.remove(peer_token.0);
        self.links.remove(&peer_token);
      }
      None => {}
    }
  }
}
