FROM rust:latest

WORKDIR /usr/src

RUN USER=root cargo new hakase

COPY Cargo.toml Cargo.lock /usr/src/hakase/

WORKDIR /usr/src/hakase

RUN cargo build --release

COPY src /usr/src/hakase/src/

RUN cargo build --release \
  && mv target/release/hakase /usr/bin \
  && rm -rf /usr/src/hakase

CMD ["/usr/bin/hakase"]
