#!/bin/sh

echo "Starting supervisord job with the following command:"
grep "command" /etc/supervisor/conf.d/supervisord.conf

# Run prep-cloudwatch-config.sh
echo "running prep-cloudwatch-config.sh"
/app/prep-cloudwatch-config.sh

# Check for existing log file, if it exists, archive it with the current date
if [ -f /app/log/node.log ]; then
  echo "Existing node.log found"
  archived_log_name="/app/log/node.log.$(date +%Y-%m-%d-%H-%M-%S)"
  mv /app/log/node.log "$archived_log_name"
  echo "renamed node.log to $archived_log_name"
fi

# Run supervisord
/usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf