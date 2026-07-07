#!/bin/sh
printf "%s" "$MONITOR_PW" > /etc/prometheus/metrics_pw.txt

envsubst < /etc/prometheus/prometheus.yml > /tmp/prometheus.yml \
  && mv /tmp/prometheus.yml /etc/prometheus/prometheus.yml

exec /bin/prometheus \
  --config.file=/etc/prometheus/prometheus.yml \
  --web.config.file=/etc/prometheus/web-config.yml