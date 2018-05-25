# mtproxy

🚧 *Work in progress*

`mio` powered Telegram MTProto proxy server.

## Installation

You can use the `cargo install` command:

    $ cargo install mtproxy

or a classic build and run:

```bash
$ git clone https://github.com/dotcypress/mtproxy
$ cd mtproxy
$ cargo build --release
$ cp target/release/mtproxy ~/.bin # assuming .bin is in your path
```

## Docker

`docker run -p 1984:1984 -dti dotcypress/mtproxy -s 'proxy secret'`
`docker logs %container_id%`