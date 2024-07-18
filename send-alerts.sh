#!/bin/sh

set -e

# prepare
cp send-alerts.crontab /etc/cron.d/send-alerts
chmod 0644 /etc/cron.d/send-alerts
crontab /etc/cron.d/send-alerts

# execute
touch /var/log/cron.log
cron
tail -f /var/log/cron.log