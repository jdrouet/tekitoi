FROM rust:1.58-alpine3.14 AS builder

RUN apk add --no-cache musl-dev

ENV USER=root

WORKDIR /code
RUN cargo init --bin && mkdir -p /code/.cargo
COPY Cargo.toml Cargo.lock /code/
RUN cargo vendor > /code/.cargo/config.toml

COPY src /code/src
COPY templates /code/templates
RUN cargo build --release --offline

FROM alpine:3.14

ENV HOST=0.0.0.0
ENV PORT=3000

COPY --from=builder /code/target/release/tekitoi-server /usr/bin/tekitoi

ENTRYPOINT ["/usr/bin/tekitoi"]