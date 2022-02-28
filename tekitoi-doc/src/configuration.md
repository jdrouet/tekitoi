# Configuration

Tekitoi can be both configured though environment variables and a configuration file.

Here is a fully filled configuration file.

```toml
# Base url to reach Tekitoi. It's used internally to build
# the redirect url sent to any provider
# It can also be specified using the BASE_URL environment variable.
base_url = "http://localhost:3000"
# The level of logging that will be user. Can be info, debug, warn, error or trace.
# It can also be specified using the LOG_LEVEL environment variable.
log_level = "info"

[cache]
# The url to reach the redis cache.
# It can also be specified using the CACHE__URL environment variable.
url = "redis://localhost"

# This is a dictionnary of all the configured clients that tekitoi can serve.
# You can add as many as you need.
[clients.client_name]
# The client ID that the oauth client will need to use
client_id = "something"
# A set of client secrets that the oauth client will need to use
client_secrets = ["foo", "bar"]
# The redirect uri that the oauth client will need to use
redirect_uri = "http://localhost:8080/api/redirect"

# This is a set of providers that you can configure
[clients.client_name.providers.github]
client_id = "github-client-id"
client_secret = "github-client-secret"
# scopes = []
# auth_url = ""
# token_url = ""
# base_api_url = ""

[clients.client_name.providers.gitlab]
client_id = "github-client-id"
client_secret = "github-client-secret"
scopes = ["read_user"]
# auth_url = ""
# token_url = ""
# base_api_url = ""

[clients.client_name.providers.google]
client_id = "google-client-id"
client_secret = "google-client-secret"
scopes = ["openid", "email", "profile"]
# auth_url = ""
# token_url = ""
# base_api_url = ""
```

