#!/bin/bash

if [ -f .env ]; then
  while IFS='=' read -r key value; do
    # Skip comment lines and empty lines
    if [[ ! "$key" =~ ^#.* ]] && [[ -n "$key" ]]; then
      export "$key"="$value"
    fi
  done < .env
fi

cargo watch -x run