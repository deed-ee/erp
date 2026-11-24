FROM rust:1.100.0-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY services/ services/
RUN cargo build --locked --release -p article-delivery

FROM debian:bookworm-20261104-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc
COPY --from=build /src/target/release/article-delivery /usr/local/bin/article-delivery
USER 10001:10001
ENTRYPOINT ["/usr/local/bin/article-delivery"]
