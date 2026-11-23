# syntax=docker/dockerfile:1.7
FROM rust:1-bookworm AS builder
WORKDIR /app
# 依存のみ先にビルドしてレイヤキャッシュを効かせる
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs \
    && cargo build --release && rm -rf src
COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs src/lib.rs && cargo build --release

FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app
COPY --from=builder /app/target/release/api /app/api
ENV BIND_ADDR=0.0.0.0:8080 LOG_FORMAT=json
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/app/api"]
