<h1 align="center">👋 Welcome to tekitoi 🦀 </h1>
<p>
  <img alt="Version" src="https://img.shields.io/badge/version-0.1.0-blue.svg?cacheSeconds=2592000" />
  <a href="#" target="_blank">
    <img alt="License: MIT" src="https://img.shields.io/badge/License-MIT-yellow.svg" />
  </a>
</p>

> A simple and lightweight proxy oauth 🦀

### 🏠 [Homepage](https://github.com/jdrouet/tekitoi)

## ✨ Features

- Github and Gitlab oauth2 proxy
- Multi arch (AMD64, i386, ARM64)
- Lightweight (Only needs 2Mo of RAM against 512Mo minimum for Keycloak)

## 🐟 Example

Command to start tekitoi

`tekitoi --config tekitoi.toml`

Configuration file

```toml
# base url on wich the tekitoi service can be reached
base_url = "https://auth.myservice.com"
# Log level, can be INFO, DEBUG, WARN, ERROR, TRACE
log_level = "info"

[database]
type = "sqlite"
url = "sqlite::memory:"

# Here you can specify a list of potential clients
[applications.client_name]
client_id = "something"
client_secrets = ["foo", "bar"]
redirect_uri = "http://localhost:8080/api/redirect"

# For each client, you can specify a set of providers (github and gitlab for now).
[applications.client_name.providers.github]
client_id = "github-client-id"
client_secret = "github-client-secret"
scopes = ["..."]
# Authorization url for this provider. Has a default value.
# auth_url = ""
# Token url for this provider. Has a default value.
# token_url = ""
# Base api use for this provider. Has a default value.
# base_api_url = ""
```

Then you can just configure your oauth2 clients with the `client_id` and the `client_secret` you just specified.

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

---

_This README was generated with ❤️ by [readme-md-generator](https://github.com/kefranabg/readme-md-generator)_
