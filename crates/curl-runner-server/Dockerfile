# Build context is the repo root:
#   docker build -f crates/curl-runner-server/Dockerfile -t curl-runner-server .
#
# The builder always runs on the build host's architecture and cross-compiles
# to a static musl binary for TARGETARCH, so multi-arch builds never compile
# Rust under QEMU emulation. ring (rustls' crypto provider) has C and assembly,
# so zig, via cargo-zigbuild, is the C cross compiler and linker for both
# targets. The migrations are embedded in the binary.
FROM --platform=$BUILDPLATFORM rust:1.99.1-slim AS builder
ARG BUILDARCH
ARG TARGETARCH
ARG ZIG_VERSION=0.14.1
ARG CARGO_ZIGBUILD_VERSION=0.24.1
RUN set -eu; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates curl xz-utils; \
    rm -rf /var/lib/apt/lists/*; \
    case "$BUILDARCH" in \
      amd64) zig_arch=x86_64; sha=24aeeec8af16c381934a6cd7d95c807a8cb2cf7df9fa40d359aa884195c4716c ;; \
      arm64) zig_arch=aarch64; sha=f7a654acc967864f7a050ddacfaa778c7504a0eca8d2b678839c21eea47c992b ;; \
      *) echo "unsupported BUILDARCH: $BUILDARCH" >&2; exit 1 ;; \
    esac; \
    tarball="zig-$zig_arch-linux-$ZIG_VERSION.tar.xz"; \
    curl -fsSL -o "/tmp/$tarball" "https://ziglang.org/download/$ZIG_VERSION/$tarball"; \
    echo "$sha  /tmp/$tarball" | sha256sum -c -; \
    mkdir -p /opt/zig; \
    tar -xJf "/tmp/$tarball" -C /opt/zig --strip-components=1; \
    rm "/tmp/$tarball"; \
    ln -s /opt/zig/zig /usr/local/bin/zig; \
    zig version
RUN cargo install cargo-zigbuild --version "$CARGO_ZIGBUILD_VERSION" --locked
WORKDIR /src
RUN rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
COPY . .
RUN set -eu; \
    case "$TARGETARCH" in \
      amd64) triple=x86_64-unknown-linux-musl ;; \
      arm64) triple=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    cargo zigbuild --release --locked -p curl-runner-server --target "$triple"; \
    install -D "target/$triple/release/curl-runner-server" /out/curl-runner-server

FROM scratch
COPY --from=builder /out/curl-runner-server /curl-runner-server
USER 65532:65532
ENV PORT=8080
EXPOSE 8080
ENTRYPOINT ["/curl-runner-server"]
