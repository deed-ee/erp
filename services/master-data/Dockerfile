FROM rust:1.100.0-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea AS chef
RUN cargo install cargo-chef --version 0.1.78 --locked
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path /recipe.json

FROM chef AS build
COPY --from=planner /recipe.json /recipe.json
RUN cargo chef cook --locked --release -p master-data --recipe-path /recipe.json
COPY . .
RUN cargo build --locked --release -p master-data

FROM debian:bookworm-20261104-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc
# Remove this layer once the pinned base ships the CVE-2025-6020 fix.
RUN apt-get update \
    && apt-get install -y --no-install-recommends --only-upgrade libpam-modules libpam-modules-bin libpam-runtime libpam0g \
    && dpkg --compare-versions "$(dpkg-query -W -f='${Version}' libpam-modules)" ge 1.5.2-6+deb12u3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/master-data /usr/local/bin/master-data
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/master-data"]
