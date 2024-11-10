FROM debian:12.7-slim as builder

# This installs all dependencies that we need (besides Rust).
RUN apt update -y && \
    apt install build-essential git clang curl libssl-dev llvm libudev-dev make cmake protobuf-compiler -y

# This installs Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rust_install.sh && chmod u+x rust_install.sh && ./rust_install.sh -y

ADD . ./workdir
WORKDIR "/workdir"

# This installs the right toolchain
RUN $HOME/.cargo/bin/rustup show

# This builds the binary.
RUN $HOME/.cargo/bin/cargo build --locked --release

# Create output folder
RUN mkdir -p output

VOLUME ["/output"]
CMD cp ./target/release/avail-node /output
