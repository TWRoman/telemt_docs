# syntax=docker/dockerfile:1

ARG TARGETARCH
ARG BINARY_AMD64
ARG BINARY_ARM64

# ==========================
# Minimal Image
# ==========================
FROM debian:12-slim AS minimal

ARG TARGETARCH
ARG BINARY_AMD64
ARG BINARY_ARM64

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        binutils \
        curl \
        xz-utils \
        ca-certificates; \
    rm -rf /var/lib/apt/lists/*

# --- Select correct binary ---
RUN set -eux; \
    case "${TARGETARCH}" in \
        amd64) BIN="${BINARY_AMD64}" ;; \
        arm64) BIN="${BINARY_ARM64}" ;; \
        *) echo "Unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    echo "Using binary: $BIN"; \
    test -f "$BIN"; \
    cp "$BIN" /telemt

# --- Install UPX (arch-aware) ---
RUN set -eux; \
    case "${TARGETARCH}" in \
        amd64) UPX_ARCH="amd64" ;; \
        arm64) UPX_ARCH="arm64" ;; \
        *) echo "Unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    \
    curl -fL \
        --retry 5 \
        --retry-delay 3 \
        --connect-timeout 10 \
        --max-time 120 \
        -o /tmp/upx.tar.xz \
        "https://github.com/telemt/telemt/releases/download/toolchains/upx-${UPX_ARCH}_linux.tar.xz"; \
    \
    tar -xf /tmp/upx.tar.xz -C /tmp; \
    install -m 0755 /tmp/upx*/upx /usr/local/bin/upx; \
    rm -rf /tmp/upx*

# --- Optimize binary ---
RUN set -eux; \
    test -f /telemt; \
    strip --strip-unneeded /telemt || true; \
    upx --best --lzma /telemt || true

# ==========================
# Debug Image
# ==========================
FROM debian:12-slim AS debug

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        tzdata \
        curl \
        iproute2 \
        busybox; \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=minimal /telemt /app/telemt
COPY config.toml /app/config.toml

EXPOSE 443 9090 9091

ENTRYPOINT ["/app/telemt"]
CMD ["config.toml"]

# ==========================
# Production Distroless on MUSL
# ==========================
FROM gcr.io/distroless/static-debian12 AS prod

WORKDIR /app

COPY --from=minimal /telemt /app/telemt
COPY config.toml /app/config.toml

USER nonroot:nonroot

EXPOSE 443 9090 9091

ENTRYPOINT ["/app/telemt"]
CMD ["config.toml"]
