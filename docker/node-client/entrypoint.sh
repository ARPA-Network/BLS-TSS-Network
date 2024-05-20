#!/bin/sh

echo "Starting supervisord job with the following command:"
grep "command" /etc/supervisor/conf.d/supervisord.conf

echo "running prep-cloudwatch-config.sh"
/app/prep-cloudwatch-config.sh

# Run supervisord
/usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf