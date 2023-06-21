FROM rust:latest

# Define working directory
WORKDIR /usr/src/app

# Copy contracts into container
COPY ../../contracts .

# Copy entrypoint.sh script and make it executable
COPY ./docker/localnet/contract-init/entrypoint.sh /usr/src/app/
RUN chmod +x /usr/src/app/entrypoint.sh

# Install dependencies
RUN apt-get update && \
    apt-get install -y git neovim protobuf-compiler libprotobuf-dev pkg-config libssh-dev build-essential net-tools netcat

# Install Foundry and start Anvil
RUN curl -L https://foundry.paradigm.xyz | bash && \
    /root/.foundry/bin/foundryup

# Create a directory for the external .env file
RUN mkdir -p /usr/src/app/external

# Run entrypoint script and keep container running
ENTRYPOINT ["/usr/src/app/entrypoint.sh"]