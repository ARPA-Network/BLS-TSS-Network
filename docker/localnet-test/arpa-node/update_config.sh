#!/bin/sh

SOURCE_CONFIG_FILE="/usr/src/app/external/config.yml"
TARGET_CONFIG_FILE="/usr/src/app/config.yml"

# Copy the source config file to the target location
cp "$SOURCE_CONFIG_FILE" "$TARGET_CONFIG_FILE"

# Figure out the IP address of the current docker container.
IP=$(hostname -i)
PORT=50061

# Check if the line exists in the file
if grep -q "node_advertised_committer_rpc_endpoint:" "$TARGET_CONFIG_FILE"; then
  # Modify the existing line (remove everything after the colon and append the new value)
  sed -i "s/\(node_advertised_committer_rpc_endpoint:\).*/\1 \"$IP:$PORT\"/" "$TARGET_CONFIG_FILE"
else
  # Append the line to a temporary file
  echo "node_advertised_committer_rpc_endpoint: \"$IP:$PORT\"" > /tmp/temp_config.yml
  # Concatenate target config file with the temporary file
  cat "$TARGET_CONFIG_FILE" >> /tmp/temp_config.yml
  # Overwrite the target config file with the new content
  cp /tmp/temp_config.yml "$TARGET_CONFIG_FILE"
  # Remove temporary file
  rm /tmp/temp_config.yml
fi