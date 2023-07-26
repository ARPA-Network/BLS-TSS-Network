# Define your base image
FROM rust:latest
SHELL ["/bin/bash", "-c"]

EXPOSE 50061-50220

EXPOSE 8545

# Set up environment variables for rust and Python
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
ENV PYTHONUNBUFFERED=1

# Prepare system
RUN apt-get update -y && apt-get install -y curl build-essential python3 python3-pip

RUN apt-get update && \
    apt-get install -y protobuf-compiler libprotobuf-dev pkg-config libssh-dev build-essential lsof findutils supervisor neovim git net-tools netcat

# Anvil and other custom steps
RUN curl -L https://foundry.paradigm.xyz | bash
RUN source /root/.bashrc
ENV PATH="~/.foundry/bin:${PATH}"
RUN foundryup

# Set up Python 3.8 and dependencies
RUN python3 -m pip install --upgrade pip
COPY requirements.txt /app/
WORKDIR /app/
RUN pip install -r requirements.txt

CMD ["tail", "-f", "/dev/null"]

