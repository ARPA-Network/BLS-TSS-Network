FROM ghcr.io/foundry-rs/foundry:latest
EXPOSE 8545
RUN apk add --no-cache bash
ENTRYPOINT ["anvil", "--host", "0.0.0.0", "--block-time", "1", "--prune-history"]
# ENTRYPOINT ["anvil", "--host", "0.0.0.0"]

