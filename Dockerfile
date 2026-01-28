FROM rust:1.92-trixie as builder

WORKDIR /usr/src/app

RUN apt update -y && \
    apt install -y --no-install-recommends \
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

RUN cargo fetch \
  && cargo build --release

RUN rm -rf /usr/local/cargo/git && \
    rm -rf /usr/local/cargo/registry

FROM debian:trixie-slim AS runner

RUN apt update && \
    apt install -y --no-install-recommends \
        tini \
        gosu \
        curl \
        libc6 \
        libgcc-s1 \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --system --gid 1001 appuser \
  && useradd --system --uid 1001 --gid appuser --home /home/appuser --shell /usr/sbin/nologin appuser \
  && mkdir -p /home/appuser/.cache \
  && chown -R 1001:1001 /home/appuser

WORKDIR /app  

COPY --from=builder /usr/src/app/target/release/ord ./ord

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENV RUST_BACKTRACE=1
ENV RUST_LOG=info

EXPOSE 3333

ENTRYPOINT ["/usr/bin/tini", "--", "/entrypoint.sh"]
CMD ["/app/ord"]

