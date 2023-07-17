#!/bin/bash

# Usage: This script is used to update the config files for the node and the docker containers. It uses the following env variables from the CDK outputs.

# Make sure these env variables are set before running this script.
# export ETH_RPC_URL="http://18.118.32.254:8545"
# export NODE_RPC_IP="18.224.44.15"

if [ -z "${ETH_RPC_URL}" ] || [ -z "${NODE_RPC_IP}" ]; then
    echo "Both ETH_RPC_URL and NODE_RPC_IP must be set before running this script."
    exit 1
fi

for config_file in config_1.yml config_2.yml config_3.yml; do
    # Get the prior values
    provider_endpoint_prior=$(grep "provider_endpoint:" "${config_file}")
    node_advertised_committer_rpc_endpoint_prior=$(grep "node_advertised_committer_rpc_endpoint:" "${config_file}")

    # Update provider_endpoint in configuration file with the value of ETH_RPC_URL
    sed -i "s|provider_endpoint:.*|provider_endpoint: \"${ETH_RPC_URL}\"|" "${config_file}"
    # Update node_advertised_committer_rpc_endpoint in configuration file with the value of NODE_RPC_URL
    sed -i "s|\(node_advertised_committer_rpc_endpoint: \).*:\([0-9]*\)|\1\"${NODE_RPC_IP}:\2|" "${config_file}"

    # Get the updated values
    provider_endpoint_updated=$(grep "provider_endpoint:" "${config_file}")
    node_advertised_committer_rpc_endpoint_updated=$(grep "node_advertised_committer_rpc_endpoint:" "${config_file}")

    # Print the prior and updated values
    echo "${config_file}:"
    echo "Prior values:"
    echo " . ${provider_endpoint_prior}"
    echo " . ${node_advertised_committer_rpc_endpoint_prior}"
    echo "Updated values:"
    echo " . ${provider_endpoint_updated}"
    echo " . ${node_advertised_committer_rpc_endpoint_updated}"
    echo "-----"
done