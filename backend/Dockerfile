# syntax=docker/dockerfile:1.7
# api と batch の 2 バイナリを 1 イメージに含める (ENTRYPOINT/command で切り替え)
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations
# BuildKit のキャッシュマウントで依存クレートとビルド成果物を再利用する
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked -p api -p batch \
    && mkdir -p /out && cp target/release/api target/release/batch /out/

FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app
COPY --from=builder /out/api /out/batch /app/
ENV BIND_ADDR=0.0.0.0:8080 LOG_FORMAT=json
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/app/api"]
