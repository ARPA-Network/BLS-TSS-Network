#!/bin/sh

# Copy the config file out of the volume mount
cp /usr/src/app/external/config.yml /usr/src/app/config.yml

echo "Starting supervisord job with the following command:"
grep "command" /etc/supervisor/conf.d/supervisord.conf

# Run supervisord
/usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf