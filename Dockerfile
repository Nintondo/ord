FROM rust:1.86.0-bookworm as builder

WORKDIR /usr/src/app

RUN apt update -y && \
    apt install -y \
    pkg-config \
    libssl-dev \
    git \
    build-essential \
    clang \
    libclang-dev \
    protobuf-compiler && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo fetch && cargo build --release

RUN rm -rf ~/.cargo/git && \
    rm -rf ~/.cargo/registry

FROM ubuntu:24.04 AS runner

WORKDIR /app

RUN apt update -y && \
    apt install -y curl openssl libc6 libgcc-s1 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/ord ./ord

ENV RUST_BACKTRACE=1
ENV RUST_LOG=info

EXPOSE 3333

ENTRYPOINT ["./ord"]
