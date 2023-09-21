#!/bin/sh

# # Copy the config file out of the volume mount
# cp /app/external/config.yml /app/config.yml

# # copy the log file file out of the volume mount
# cp /app/external/log4js.json /app/log4js.json
# # copy the sqlite file out of the volume mount
# cp /app/external/arpa.db /app/arpa.db

echo "Starting supervisord job with the following command:"
grep "command" /etc/supervisor/conf.d/supervisord.conf

# Run supervisord
/usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf