FROM rust:latest

# expose the two RPC ports
EXPOSE 50061
EXPOSE 50091

# Define working directory
WORKDIR /usr/src/app

# Install dependencies
RUN apt-get update && \
    apt-get install -y protobuf-compiler libprotobuf-dev pkg-config libssh-dev build-essential supervisor neovim git net-tools netcat

# Clone and build Randcast Node Client
RUN git clone https://github.com/ARPA-Network/BLS-TSS-Network.git && \
    cd BLS-TSS-Network && \
    cargo build

# Create a directory for the external config.yml file
RUN mkdir -p /usr/src/app/external

# Copy supervisord configuration file
COPY supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Copy and run update_config.sh script
COPY update_config.sh /usr/src/app/
RUN chmod +x /usr/src/app/update_config.sh

# Copy entrypoint.sh script
COPY entrypoint.sh /usr/src/app/
RUN chmod +x /usr/src/app/entrypoint.sh

ENTRYPOINT ["/usr/src/app/entrypoint.sh"]

# debugging
# docker run -d --network randcast_network --name node3 -v ./docker/arpa-node/config_3.yml:/usr/src/app/external/config.yml -it --entrypoint /bin/bash arpa-node:latest