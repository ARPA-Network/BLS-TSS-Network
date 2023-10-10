# Build stage
FROM alpine:3.18 as builder

# Define working directory
WORKDIR /app

# Install dependencies
RUN apk add curl build-base perl git protoc protobuf-dev pkgconfig openssl
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh \
    && chmod +x ./rustup.sh \
    && ./rustup.sh -y --default-toolchain none

# Force cargo to fetch with git in order to avoid memory issues when running cargo build
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# Copy the source code and build Randcast
COPY . .
RUN source $HOME/.profile && cargo build --release

####### Final Stage ########
FROM docker.io/frolvlad/alpine-glibc:alpine-3.16_glibc-2.34

# Create a directory for the external config.yml file
RUN mkdir -p /data

# Copy files from builder stage
COPY --from=builder /app/target/release/node-shell /usr/local/bin/node-shell

ENTRYPOINT ["/bin/sh", "-c"]