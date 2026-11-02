# Build context is the repo root:
#   docker build -f crates/curl-runner-server/Dockerfile -t curl-runner-server .
#
# The builder always runs on the build host's architecture and cross-compiles
# to a static musl binary for TARGETARCH, so multi-arch builds never compile
# Rust under QEMU emulation. The server has no C dependencies, so rust-lld is
# enough to link both targets.
FROM --platform=$BUILDPLATFORM rust:1.99.1-slim AS builder
ARG TARGETARCH
WORKDIR /src
RUN rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
COPY . .
RUN set -eu; \
    case "$TARGETARCH" in \
      amd64) triple=x86_64-unknown-linux-musl ;; \
      arm64) triple=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    RUSTFLAGS="-C linker=rust-lld" \
      cargo build --release --locked -p curl-runner-server --target "$triple"; \
    install -D "target/$triple/release/curl-runner-server" /out/curl-runner-server

FROM scratch
COPY --from=builder /out/curl-runner-server /curl-runner-server
USER 65532:65532
ENV PORT=8080
EXPOSE 8080
ENTRYPOINT ["/curl-runner-server"]
