#!/bin/sh

set -e

# prepare
cp confirm-alive.crontab /etc/cron.d/confirm-alive
chmod 0644 /etc/cron.d/confirm-alive
crontab /etc/cron.d/confirm-alive

# execute
touch /var/log/cron.log
cron
tail -f /var/log/cron.log