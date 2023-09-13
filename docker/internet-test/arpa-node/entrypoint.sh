#!/bin/sh

# Copy the config file out of the volume mount
cp /usr/src/app/external/config.yml /usr/src/app/config.yml
# copy the log file file out of the volume mount
cp /usr/src/app/external/log4js.json /usr/src/app/log4js.json
# copy the sqlite file out of the volume mount
cp /usr/src/app/external/arpa.db /usr/src/app/arpa.db

echo "Starting supervisord job with the following command:"
grep "command" /etc/supervisor/conf.d/supervisord.conf

# Run supervisord
/usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf