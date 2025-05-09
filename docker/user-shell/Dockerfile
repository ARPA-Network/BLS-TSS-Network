# Build stage
FROM alpine:3.18 as builder

# Define working directory
WORKDIR /app

# Install dependencies
RUN apk add curl build-base perl git protoc protobuf-dev pkgconfig openssl

# Install rustup but don't install any toolchain yet
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh \
    && chmod +x ./rustup.sh \
    && ./rustup.sh -y --default-toolchain none

# Add cargo to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Force cargo to fetch with git in order to avoid memory issues when running cargo build
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# Copy the source code and build Randcast
COPY . .

# Install the toolchain specified in rust-toolchain file
RUN rustup show

# Now build the project
RUN cargo build --release

####### Final Stage ########
FROM ghcr.io/foundry-rs/foundry:latest

# Switch to root user temporarily to create directories
USER root

# Create a directory for the external config.yml file
RUN mkdir -p /data && chown foundry:foundry /data

# Copy files from builder stage
COPY --from=builder /app/target/release/user-shell /usr/local/bin/user-shell

# Switch back to foundry user
USER foundry

ENTRYPOINT ["/bin/sh", "-c"]