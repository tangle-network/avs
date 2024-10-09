#!/bin/sh
exec /usr/local/bin/{{project-name}} run \
      --blueprint-id="$BLUEPRINT_ID" \
      --service-id="$SERVICE_ID" \
      --bind-addr="$BIND_ADDR" \
      --bind-port="$BIND_PORT" "$@"