##########################################################
# Randcast Builder
##########################################################
FROM alpine:3.18 as build_randcast

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
# CW Agent Builder
##########################################################
FROM debian:latest as build_cw_agent

# Define working directory
WORKDIR /app

# install dependencies
RUN apt-get update && apt-get install -y dpkg wget vim
RUN wget https://amazoncloudwatch-agent.s3.amazonaws.com/debian/amd64/latest/amazon-cloudwatch-agent.deb
RUN dpkg -i amazon-cloudwatch-agent.deb


##########################################################
# Final stage
##########################################################
FROM docker.io/frolvlad/alpine-glibc:alpine-3.16_glibc-2.34

# install dependencies
RUN apk add supervisor curl vim bash 

##################
# Randcast Stuff #
##################
# Copy the built binary from the randcast builder stage
COPY --from=build_randcast /app/target/release/node-client /app/node-client
COPY --from=build_randcast /app/target/release/node-config-checker /usr/local/bin/node-config-checker

##################
# CW Agent Stuff #
##################
# Copy cloudwatch agent files from the builder stage
COPY --from=build_cw_agent /tmp /tmp
COPY --from=build_cw_agent /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build_cw_agent /opt/aws/amazon-cloudwatch-agent /opt/aws/amazon-cloudwatch-agent
ENV RUN_IN_CONTAINER="True"

# Copy the AWS config file to the container
COPY docker/node-client/config /root/.aws/config
RUN chmod 600 /root/.aws/config

# Grab cloudwatch push token from secret mount 
RUN --mount=type=secret,id=cloudwatch_upload_token \
  cat /run/secrets/cloudwatch_upload_token > /root/.aws/credentials

# Copy the cloudwatch agent configuration file to the container
COPY docker/node-client/cloudwatch-agent-config.json /opt/aws/amazon-cloudwatch-agent/bin/default_linux_config.json

# copy prep-cloudwatch-config.sh script to the container
COPY docker/node-client/prep-cloudwatch-config.sh /app/prep-cloudwatch-config.sh
RUN chmod +x /app/prep-cloudwatch-config.sh

###############
#  Entrypoint # 
###############
# Copy supervisord configuration file
COPY docker/node-client/supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Copy entrypoint.sh script
COPY docker/node-client/entrypoint.sh /app/
RUN chmod +x /app/entrypoint.sh

ENTRYPOINT ["/app/entrypoint.sh"]
# ENTRYPOINT ["tail", "-f", "/dev/null"]
