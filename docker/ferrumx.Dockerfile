# Node for FerrumX Network
#
# Refer README for more information

FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /ferrum-x-network
COPY . .
RUN apt-get update -y && \
        apt-get install -y cmake curl llvm libudev-dev libgmp3-dev protobuf-compiler pkg-config libssl-dev git gcc build-essential clang libclang-dev

# Install rust wasm. Needed for substrate wasm engine
RUN rustup target add wasm32-unknown-unknown

RUN cargo build --locked --release

# This is the 2nd stage: a very small image where we copy the binary."
FROM docker.io/library/ubuntu:20.04
LABEL description="Docker image for FerrumXNode" \
  image.type="builder" \
  image.authors="Ferrum Network" 

# Copy the node binary.
COPY --from=builder /ferrum-x-network/target/release/ferrum-x-network /usr/local/bin

RUN useradd -m -u 1000 -U -s /bin/sh -d /node-dev node-dev && \
  mkdir -p /chain-data /node-dev/.local/share && \
  chown -R node-dev:node-dev /chain-data && \
  ln -s /chain-data /node-dev/.local/share/ferrum-x-network && \
  # unclutter and minimize the attack surface
  rm -rf /usr/bin /usr/sbin && \
  # check if executable works in this container
  /usr/local/bin/ferrum-x-network --help

USER node-dev

# 30333 for substrate p2p
# 9933 for RPC call
# 9944 for Websocket
# 9615 for Prometheus (metrics)
EXPOSE 30333 9933 9944 9615
VOLUME ["/chain-data"]

ENTRYPOINT ["/usr/local/bin/ferrum-x-network"]