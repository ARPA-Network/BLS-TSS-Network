# # Build stage
# FROM rust:latest as builder

# # Define working directory
# WORKDIR /app

# # Install dependencies
# RUN apt-get update && \
#     apt-get install -y protobuf-compiler libprotobuf-dev pkg-config libssh-dev build-essential

# # Force cargo to fetch with git in order to avoid memory issues when running cargo build
# ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# # Clone and build Randcast Node Client
# RUN git clone https://github.com/ARPA-Network/BLS-TSS-Network.git && \
#     cd BLS-TSS-Network && \
#     cargo build --release

##########################################################
# Final stage
FROM rust:latest

RUN apt-get update && \
    apt-get install -y supervisor 

# expose the two RPC ports
EXPOSE 50061
EXPOSE 50091

# Create a directory for the external config.yml file
RUN mkdir -p /app/external

# ! (old) Copy the built binary from the builder stage
# COPY --from=builder /app/BLS-TSS-Network/target/release/node-client /app/node-client
COPY node-client /app/node-client

# Copy supervisord configuration file
COPY supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Copy entrypoint.sh script
COPY entrypoint.sh /app/
RUN chmod +x /app/entrypoint.sh

ENTRYPOINT ["/app/entrypoint.sh"]
# ENTRYPOINT ["tail", "-f", "/dev/null"]
