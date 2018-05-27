#![deny(warnings)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

extern crate bytes;
extern crate clap;
extern crate crypto;
extern crate env_logger;
extern crate mio;
extern crate rand;
extern crate slab;

mod pool;
mod proto;
mod proxy;
mod pump;

use std::io;

use clap::{App, Arg};
use proxy::Server;

fn main() -> Result<(), io::Error> {
  env_logger::init();

  let args = App::new("mtproxy")
    .version(env!("CARGO_PKG_VERSION"))
    .author("Vitaly Domnikov <dotcypress@gmail.com>")
    .about("MTProto proxy server.")
    .arg(
      Arg::with_name("seed")
        .value_name("SEED")
        .short("s")
        .long("seed")
        .help("Proxy secret seed.")
        .takes_value(true)
        .required(true)
        .display_order(0),
    )
    .arg(
      Arg::with_name("addres")
        .value_name("ADDRESS")
        .short("a")
        .long("addr")
        .help("Listening address. Default value: 0.0.0.0:1984.")
        .takes_value(true)
        .display_order(1),
    )
    .get_matches();

  let seed = args.value_of("seed").unwrap();
  let addr = args.value_of("addres").unwrap_or("0.0.0.0:1984");
  let addr = String::from(addr)
    .parse()
    .expect(&format!("Not supported address: {}", addr));

  let mut serv = Server::new(addr, seed);
  println!("Secret: {}\n", serv.secret());
  println!("Ip:     {}", addr.ip());
  println!("Port:   {}", addr.port());
  serv.run()
}
