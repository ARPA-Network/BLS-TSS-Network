####### Builder Stage ########
FROM rust:latest as builder

# Define working directory
WORKDIR /usr/src/app

# Install dependencies
RUN apt-get update && \
    apt-get install -y protobuf-compiler libprotobuf-dev pkg-config libssh-dev build-essential supervisor neovim git net-tools netcat

# Force cargo to fetch with git in order to avoid memory issues when running cargo build
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# Clone and build Randcast Node Client
RUN git clone https://github.com/ARPA-Network/BLS-TSS-Network.git && \
    cd BLS-TSS-Network && \
    cargo build --release

####### Final Stage ########
FROM rust:latest

# expose the anvil port
EXPOSE 8545 

# Create a directory for the external config.yml file
RUN mkdir -p /usr/src/app/external


# Copy files from builder stage
COPY --from=builder /usr/src/app/BLS-TSS-Network/target/release/user-shell /usr/src/app/user-shell

# # add user-shell to the path
# RUN echo "export PATH=$PATH:/usr/src/app/user-shell" >> ~/.bashrc

RUN apt-get update && \
    apt-get install -y protobuf-compiler libprotobuf-dev pkg-config libssh-dev build-essential neovim git net-tools netcat

# Install Foundry
RUN curl -L https://foundry.paradigm.xyz | bash && \
    /root/.foundry/bin/foundryup

ENTRYPOINT ["/bin/bash", "-c"]