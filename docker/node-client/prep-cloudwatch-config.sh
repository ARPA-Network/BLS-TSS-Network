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
# Pint updated CW Config to console #
#####################################

echo "Printing updated CloudWatch configuration file at: /opt/aws/amazon-cloudwatch-agent/bin/default_linux_config.json"

# run cat /opt/aws/amazon-cloudwatch-agent/bin/default_linux_config.json
cat /opt/aws/amazon-cloudwatch-agent/bin/default_linux_config.json 

# exit 1


# #####################################
# # Update Log file path in CW Config #
# #####################################

# # Extract the logger.log_file_path from the app config file
# log_file_path=$(awk '/log_file_path:/ {print $2}' /app/config.yml)
# echo "Extracted log_file_path from app config: $log_file_path"

# # Replace the file_path in the CloudWatch configuration file
# sed -i "s#\"file_path\": \".*\"#\"file_path\": \"/app/$log_file_path/node.log\"#" /opt/aws/amazon-cloudwatch-agent/bin/default_linux_config.json

# echo "Updated file_path in CloudWatch configuration file."