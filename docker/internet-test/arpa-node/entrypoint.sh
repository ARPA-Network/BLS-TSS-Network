#!/bin/sh

# Copy the config file to a new location
cp /usr/src/app/external/config.yml /usr/src/app/config.yml

# Update provider_endpoint in config_new.yml with the value of RPC_ENDPOINT
sed -i "s|provider_endpoint:.*|provider_endpoint: \"$ETH_RPC_URL\"|" /usr/src/app/config.yml

# Update node_advertised_committer_rpc_endpoint in config_new.yml with the value of NODE_RPC_URL
sed -i "s|node_advertised_committer_rpc_endpoint:.*|node_advertised_committer_rpc_endpoint: \"$NODE_RPC_URL\"|" /usr/src/app/config.yml

# Print a message after updating config.yml
echo "Updated /usr/src/app/config.yml with the following lines:"
grep "provider_endpoint" /usr/src/app/config.yml
grep "node_advertised_committer_rpc_endpoint" /usr/src/app/config.yml


echo "Starting supervisord job with the following command:"
grep "command" /etc/supervisor/conf.d/supervisord.conf
echo "----------------------------------------"


# Run supervisord
/usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf