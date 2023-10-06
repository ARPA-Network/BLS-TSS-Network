#!/bin/sh

# Run update_config.sh script
/usr/src/app/update_config.sh

# Print a message after updating config.yml
echo "Updated /usr/src/app/config.yml with the following line:"
grep "node_advertised_committer_rpc_endpoint" /usr/src/app/config.yml

echo "----------------------------------------"
# Run supervisord
/usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf