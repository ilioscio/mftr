# syntax=docker/dockerfile:1
# MFTR dedicated server (M2 slice 6). Build: `docker build -t mftr-server .`
# Run:   `docker run --rm -p 7777:7777/udp mftr-server` (an ARAM server with 10 bots and
# champion select; see docs/hosting.md for every option). The server key is created in /data:
# mount a volume there to keep it.

FROM rust:1-slim-bookworm AS build
# The image's own toolchain (rust-toolchain.toml would make rustup download another).
ENV RUSTUP_TOOLCHAIN=${RUST_VERSION}
WORKDIR /src
COPY . .
# Only the server and the tools: the Godot extension isn't needed (or built) here. Behind a
# TLS-intercepting proxy, pass its CA: `docker build --secret id=ca_bundle,src=ca.crt ...`.
RUN --mount=type=secret,id=ca_bundle,required=false \
    if [ -f /run/secrets/ca_bundle ]; then export CARGO_HTTP_CAINFO=/run/secrets/ca_bundle; fi; \
    cargo build --locked --profile dist -p mftr-server -p mftr-tools
# An empty /data the unprivileged runtime user can write replays and the server key to.
RUN mkdir /data-empty

# A minimal runtime: glibc and nothing else, running as an unprivileged user.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /src/target/dist/mftr-server /src/target/dist/mftr-tools /usr/local/bin/
COPY --from=build --chown=nonroot:nonroot /data-empty /data
WORKDIR /data
EXPOSE 7777/udp
ENTRYPOINT ["/usr/local/bin/mftr-server", "--bind", "0.0.0.0:7777"]
CMD ["--scenario", "aram", "--bots", "10", "--lobby"]
