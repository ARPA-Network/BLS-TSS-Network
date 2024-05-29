cd <YOUR_ARPA_NETWORK_ROOT_DIRECTORY>

# Suppose you have a config.yml here
# Use `node-config-checker` to make the integrity check
# It will print out the address of the account provided in the configuration file,
# otherwise the error reason will be printed

docker run \
-v $(pwd)/conf/config.yml:/app/config.yml \
-v $(pwd)/opr.keystore:/app/opr.keystore ghcr.io/arpa-network/node-config-checker:latest "node-config-checker -c /app/config.yml"

# Create the necessary directories
mkdir -p db
mkdir -p log

# Create the config.yml file and fill in config details in step #1
# Pull the latest Docker image
docker pull ghcr.io/arpa-network/node-client:latest

docker run -d \
  --name arpa-node \
  -v $(pwd)/conf/config.yml:/app/config.yml \
  -v $(pwd)/opr.keystore:/app/opr.keystore \
  -v $(pwd)/db:/app/db \
  -v $(pwd)/log:/app/log \
  --network=host \
  ghcr.io/arpa-network/node-client:latest

# To login the docker container and check the stdout_log
docker exec -it arpa-node sh

vi log/node.log
