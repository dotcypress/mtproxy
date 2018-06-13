//#![deny(warnings)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate structopt;
extern crate bytes;
extern crate crypto;
extern crate mio;
extern crate rand;
extern crate reqwest;
extern crate slab;
extern crate stderrlog;

mod pool;
mod proto;
mod proxy;
mod pump;

use std::{io, net::SocketAddr};

use proxy::Server;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Cli {
  #[structopt(
    short = "a", long = "addr", default_value = "0.0.0.0:1984", help = "Listening address."
  )]
  addr: SocketAddr,
  
  #[structopt(long = "ipv6", help = "Use IPv6.")]
  ipv6: bool,

  #[structopt(short = "s", long = "seed", help = "Proxy secret seed.")]
  seed: String,

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

  let mut server = Server::new(cli.addr, &cli.seed, cli.ipv6, cli.tag);
  server.init()?;
  if !cli.quiet {
    println!("Secret: {}\n", server.secret());
    println!("Ip:     {}", cli.addr.ip());
    println!("Port:   {}", cli.addr.port());
  }
  server.run()
}
