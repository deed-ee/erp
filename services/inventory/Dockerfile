FROM rust:1.100.0-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/
COPY services/ services/
RUN cargo build --locked --release -p inventory

FROM debian:bookworm-20261104-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc
# Remove this overlay when the pinned base ships the Debian PAM fix.
RUN apt-get update \
    && apt-get install -y --no-install-recommends --only-upgrade \
        libpam-modules=1.5.2-6+deb12u3 libpam-modules-bin=1.5.2-6+deb12u3 \
        libpam-runtime=1.5.2-6+deb12u3 libpam0g=1.5.2-6+deb12u3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/inventory /usr/local/bin/inventory
USER 10001:10001
ENTRYPOINT ["/usr/local/bin/inventory"]
