#!/bin/bash

######################################
# Update Wallet Address in CW Config #
######################################
echo "########################################"
# Run node-config-checker to compute wallet address from private key in config file
address=$(node-config-checker -c /app/config.yml)
echo "Computed wallet address: $address from config file."

# Replace the log_stream_name in the CloudWatch configuration file
sed -i "s/\"log_stream_name\": \".*\"/\"log_stream_name\": $address/" /opt/aws/amazon-cloudwatch-agent/bin/default_linux_config.json

echo "Updated log_stream_name in CloudWatch configuration file."

#####################################
# Print updated CW Config to console #
#####################################

echo "Printing updated CloudWatch configuration file at: /opt/aws/amazon-cloudwatch-agent/bin/default_linux_config.json"

# run cat /opt/aws/amazon-cloudwatch-agent/bin/default_linux_config.json
cat /opt/aws/amazon-cloudwatch-agent/bin/default_linux_config.json 