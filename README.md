<h1 align="center">👋 Welcome to tekitoi 🦀 </h1>
<p>
  <img alt="Version" src="https://img.shields.io/badge/version-0.1.0-blue.svg?cacheSeconds=2592000" />
  <a href="#" target="_blank">
    <img alt="License: MIT" src="https://img.shields.io/badge/License-MIT-yellow.svg" />
  </a>
  <a href="https://codecov.io/github/jdrouet/tekitoi" >
 <img src="https://codecov.io/github/jdrouet/tekitoi/graph/badge.svg"/>
 </a>
</p>

> A simple and lightweight oauth provider 🦀

### 🏠 [Homepage](https://github.com/jdrouet/tekitoi)

## ✨ Features

- [x] Multi arch (AMD64, i386, ARM64)
- [x] Lightweight (Only needs 2Mo of RAM against 512Mo minimum for Keycloak)
- [x] Authenticate with defines profiles without passwords
- [ ] Allow to login with predefined email and password
- [ ] Allow to signup with email and password
- [ ] Facebook oauth2 proxy
- [ ] Google oauth2 proxy
- [ ] Github oauth2 proxy
- [ ] Gitlab oauth2 proxy

## 🐟 Example

Command to start tekitoi

```bash
## Without specific configuration
# with the binary
tekitoi-server
# with docker
docker run -p 3000:3000 jdrouet/tekitoi
## Fully configured
# with the binary
HOST=0.0.0.0 PORT=3000 CONFIG_PATH=./config.json DATABASE_URL=./tekitoi.db tekitoi-server
# with docker
docker run \
    -p 3020:3020 \
    -v $(pwd):/data \
    -e PORT=3020 \
    -e CONFIG_PATH=/data/config.json \
    -e DATABASE_URL=/data/tekitoi.db \
    jdrouet/tekitoi
```

This will simply start it without predifined configuration.

## 🍽️ Configuration

- `CONFIG_PATH`

The path of the configuration file that will be synchronized with the database. That's where the applications, client ids, and authentication strategies are defined.

An example can be found [here](./server/config.json).

- `DATABASE_URL`

The path of the sqlite database that will be used. The default value is `:memory:`.

When using Docker, the default value will point to `/data/tekitoi.db`.

- `HOST` and `PORT`

They refer to where the server will bind. By default `HOST=127.0.0.1` and `PORT=310`.

When using Docker, the default values are `HOST=0.0.0.0` and `PORT=3000`.


## 🐾 Roadmap

- Implement more oauth2 connectors (Google, Facebook, Twitter, you name it)
- Improve documentation
- Create openapi documentation
- Add some instrumentation

## 👤 Author

👤 **Jeremie Drouet <jeremie.drouet@gmail.com>**

- Github: [@jdrouet](https://github.com/jdrouet)
- Gitlab [@jeremie.drouet](https://gitlab.com/jeremie.drouet)

## Show your support

Give a ⭐️ if this project helped you!
