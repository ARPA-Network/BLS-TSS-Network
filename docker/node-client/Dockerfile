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

##########################################################
# Final stage
FROM docker.io/frolvlad/alpine-glibc:alpine-3.16_glibc-2.34

RUN apk add supervisor

# Create a directory for the external config.yml file
RUN mkdir -p /app/external

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/node-client /app/node-client

# Copy supervisord configuration file
COPY docker/node-client/supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Copy entrypoint.sh script
COPY docker/node-client/entrypoint.sh /app/
RUN chmod +x /app/entrypoint.sh

ENTRYPOINT ["/app/entrypoint.sh"]
# ENTRYPOINT ["tail", "-f", "/dev/null"]
