# Define your base image
FROM rust:latest as builder
EXPOSE 50061-50220 8545

# Set up environment variables for rust and Python
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN apt-get update && \
    apt-get install -y protobuf-compiler libprotobuf-dev pkg-config libssh-dev build-essential lsof findutils supervisor git net-tools python3 python3-pip

# Set up Python 3.8 and dependencies
COPY requirements.txt .
RUN python3 -m pip install --upgrade pip && \
    pip install -r requirements.txt

# Set up foundry
RUN cargo install --git https://github.com/foundry-rs/foundry --profile local --force foundry-cli anvil

CMD ["tail", "-f", "/dev/null"]
