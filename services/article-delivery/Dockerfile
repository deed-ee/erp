FROM rust:1.100.0-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY services/ services/
RUN cargo build --locked --release -p article-delivery

FROM debian:bookworm-20260421-slim@sha256:f9c6a2fd2ddbc23e336b6257a5245e31f996953ef06cd13a59fa0a1df2d5c252
COPY --from=build /src/target/release/article-delivery /usr/local/bin/article-delivery
USER 10001:10001
ENTRYPOINT ["/usr/local/bin/article-delivery"]
