FROM rust:1.92-trixie AS source

ARG CARGO_BUILD_JOBS=4

WORKDIR /usr/src/app

RUN apt update -y && \
    apt install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    git \
    build-essential \
    clang \
    libclang-dev \
    protobuf-compiler \
    python3 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo fetch --locked

FROM source AS test
RUN python3 scripts/check_deployment.py \
  && python3 scripts/test_deployment.py \
  && cargo test --locked --test integration shutdown -- --test-threads=1 \
  && cargo test --locked --test integration info -- --test-threads=1

FROM test AS builder
RUN cargo build --locked --release

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

COPY docker/healthcheck.sh /usr/local/bin/ord-healthcheck
RUN chmod 755 /usr/local/bin/ord-healthcheck
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=15s --timeout=5s --start-period=60s --retries=4 CMD ["/usr/local/bin/ord-healthcheck"]

EXPOSE 3333

ENTRYPOINT ["/usr/bin/tini", "--", "/entrypoint.sh"]
CMD ["/app/ord"]

