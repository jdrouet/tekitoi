# Installation

## Using Docker

Tekitoi is mainly distributed on [Docker Hub](https://hub.docker.com/r/jdrouet/tekitoi) and you can start it with the following command.

```bash
docker run -d \
  -e CACHE__URL=redis://redis-hostname \
  -p 3000:3000 \
  -v /path/to/config.toml:/config.toml:ro \
  jdrouet/tekitoi:latest --config /config.toml
```

You can also use it inside a [docker-compose](https://docs.docker.com/compose/) file

```yaml
services:
  cache:
    image: redis:alpine
  
  tekitoi:
    image: jdrouet/tekitoi:latest
    command: --config /config.toml
    environment:
      CACHE__URL: redis://cache
    port:
      - 3000:3000
    volumes:
      - /path/to/config.toml:/config.toml:ro
```

## From source

To compile Tekitoi from the sources, you will just need the rust suite, cargo and git

```bash
git clone https://github.com/jdrouet/tekitoi
cd tekitoi/tekitoi-server
cargo build --release
./target/release/tekitoi-server --config /path/to/config.toml
```
