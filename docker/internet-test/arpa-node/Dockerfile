FROM rust:latest
# FROM rustlang/rust:nightly

# expose the two RPC ports
EXPOSE 50061
EXPOSE 50091

# Define working directory
WORKDIR /usr/src/app

# Install dependencies
RUN apt-get update && \
    apt-get install -y protobuf-compiler libprotobuf-dev pkg-config libssh-dev build-essential supervisor neovim git net-tools netcat

ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

## this didnt work
# -RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
# +RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
#  RUN git clone https://github.com/twilight-rs/http-proxy.git
#  RUN cd http-proxy \
#      && . $HOME/.cargo/env \
# -    && cargo +nightly build --release
# +    && cargo +nightly build --release -Z sparse-registry

# Clone and build Randcast Node Client
RUN git clone https://github.com/ARPA-Network/BLS-TSS-Network.git && \
    cd BLS-TSS-Network && \
    cargo build

# Create a directory for the external config.yml file
RUN mkdir -p /usr/src/app/external

# Copy supervisord configuration file
COPY supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Copy entrypoint.sh script
COPY entrypoint.sh /usr/src/app/
RUN chmod +x /usr/src/app/entrypoint.sh

ENTRYPOINT ["/usr/src/app/entrypoint.sh"]

# debugging
# docker run -d --network randcast_network --name node3 -v ./docker/arpa-node/config_3.yml:/usr/src/app/external/config.yml -it --entrypoint /bin/bash arpa-node:latest