#!/bin/bash

# Enable build cache
docker volume create trusty-tail-cargo-registry
dokku storage:mount trusty-tail trusty-tail-cargo-registry:/usr/local/cargo/registry
docker volume create trusty-tail-cargo-target
dokku storage:mount trusty-tail trusty-tail-cargo-target:/app/target