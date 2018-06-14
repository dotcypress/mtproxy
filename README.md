# mtproxy

🚧 *Work in progress*

`mio` powered Telegram MTProto proxy server.

## Installation

You can use the `cargo install` command:

```bash
$ rustup update
$ cargo install mtproxy
```
or a classic build and run:

```bash
$ git clone https://github.com/dotcypress/mtproxy
$ cd mtproxy
$ cargo build --release
$ cp target/release/mtproxy ~/.bin # assuming .bin is in your path
```

*Note*: `mtproxy` requires rust v1.26.0 or higher.

## Docker
### Start proxy
* `docker run --name 'mtproto_proxy' --restart unless-stopped -p 1984:1984 -dti dotcypress/mtproxy -s '07123e1f482356c415f684407a3b8723'`

`07123e1f482356c415f684407a3b8723` - proxy secret, you should choose another word or generate random with `openssl rand -hex 16`

### Get secret
* `docker logs mtproto_proxy`

### Stop proxy
* `docker stop mtproto_proxy`

### Remove proxy
* `docker rm mtproto_proxy`
