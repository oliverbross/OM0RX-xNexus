# Validation tooling only. No application binary is packaged or installed.
# linux/amd64 manifest digests resolved from the official images on 2026-11-14.
FROM node:24-bookworm-slim@sha256:0e2646ab57e77b7a384a0cb59b6815b182c77e1f9c3ba36bd8435bd9c4e6680e AS node
FROM ubuntu:24.04@sha256:77eb101c607d6e3d4f5ad0c8a02c2136c689b98aba1f359e6502ba517e5c57ec
COPY --from=node /usr/local/ /usr/local/
ENV DEBIAN_FRONTEND=noninteractive RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo
ENV PATH=/opt/cargo/bin:$PATH
# Mirrors xnexus-ci.yml's Linux desktop deps plus bootstrap utilities. Ubuntu
# apt repositories are signed but rolling; retain the package inventory and
# resulting image ID, rather than claiming byte-reproducible apt resolution.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git xz-utils python3 build-essential \
    gfortran libfftw3-dev libboost-dev cmake ninja-build pkg-config \
    libasound2-dev libudev-dev libwebkit2gtk-4.1-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev libsoup-3.0-dev \
    && dpkg-query -W > /opt/packages.tsv && rm -rf /var/lib/apt/lists/*
# Same exact installer and checksum as upstream release.yml; no curl | sh.
RUN curl --proto '=https' --tlsv1.2 -sSfLo /tmp/rustup-init \
    https://static.rust-lang.org/rustup/archive/1.29.1/x86_64-unknown-linux-gnu/rustup-init \
    && echo 'dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71  /tmp/rustup-init' | sha256sum -c - \
    && chmod +x /tmp/rustup-init \
    && /tmp/rustup-init -y --default-toolchain 1.93.1 --profile minimal \
    && rm /tmp/rustup-init \
    && rustc --version && cargo --version && node --version && npm --version
WORKDIR /work
