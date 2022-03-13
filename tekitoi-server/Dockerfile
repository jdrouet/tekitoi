FROM --platform=$BUILDPLATFORM rust:1-alpine3.14 AS fetcher

ENV USER=root

WORKDIR /code
RUN cargo init --bin
COPY Cargo.toml Cargo.lock /code/
RUN mkdir -p /code/.cargo \
  && cargo vendor > /code/.cargo/config.toml

FROM rust:1-alpine3.14 AS builder

RUN apk add --no-cache musl-dev

WORKDIR /code
COPY Cargo.toml Cargo.lock /code/
COPY --from=fetcher /code/vendor /code/vendor
COPY --from=fetcher /code/.cargo /code/.cargo
COPY src /code/src
COPY templates /code/templates

RUN cargo build --release --offline

FROM alpine:3.14

ENV HOST=0.0.0.0
ENV PORT=3000
ENV STATIC_PATH=/static

COPY static /static
COPY --from=builder /code/target/release/tekitoi-server /usr/bin/tekitoi

ENTRYPOINT ["/usr/bin/tekitoi"]