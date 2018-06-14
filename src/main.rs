#![deny(warnings)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate structopt;
extern crate bytes;
extern crate crypto;
extern crate mio;
extern crate rand;
extern crate rustc_serialize;
extern crate rustls;
extern crate slab;
extern crate stderrlog;
extern crate webpki;
extern crate webpki_roots;

mod config;
mod pool;
mod proto;
mod proxy;
mod pump;

use std::{io, net::SocketAddr};

use proxy::Server;
use rustc_serialize::hex::FromHex;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Cli {
  #[structopt(
    short = "a", long = "addr", default_value = "0.0.0.0:1984", help = "Listening address."
  )]
  addr: SocketAddr,

  #[structopt(long = "ipv6", help = "Use IPv6.")]
  ipv6: bool,

  #[structopt(short = "s", long = "secret", help = "Proxy secret.")]
  secret: String,

  #[structopt(long = "tag", help = "Proxy tag.")]
  tag: Option<String>,

  #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
  verbose: usize,

  #[structopt(short = "q", long = "quiet", help = "Silence all output.")]
  quiet: bool,
}

fn main() -> Result<(), io::Error> {
  let cli = Cli::from_args();

  stderrlog::new()
    .module(module_path!())
    .quiet(cli.quiet)
    .verbosity(cli.verbose)
    .timestamp(stderrlog::Timestamp::Second)
    .init()
    .unwrap();

  let secret = match cli.secret.from_hex() {
    Ok(ref buf) if buf.len() == 16 => buf.to_vec(),
    Ok(_) => {
      return Err(io::Error::new(
        io::ErrorKind::Other,
        "Unsupported secret length",
      ))
    }
    Err(_) => return Err(io::Error::new(io::ErrorKind::Other, "Mailformed secret")),
  };

  let mut server = Server::new(cli.addr, secret, cli.ipv6, cli.tag);
  server.init()?;
  server.run()
}
