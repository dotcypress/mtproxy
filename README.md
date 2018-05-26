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
### Start proxy
* `docker run --name 'mtproto_proxy' --restart unless-stopped -p 1984:1984 -dti dotcypress/mtproxy -s 'proxy secret'`

'proxy secret' - is seed for generating secret, you should choose another word or generate random with `openssl rand -hex 15`

### Get secret
* `docker logs mtproto_proxy`

### Stop proxy
* `docker stop mtproto_proxy`

### Remove proxy
* `docker rm mtproto_proxy`
