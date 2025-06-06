#!/bin/sh

SERVER_PATH=$(nix --extra-experimental-features "nix-command flakes" build --no-link --print-out-paths .#cytrans-web-server)
STATIC_PATH=$(nix --extra-experimental-features "nix-command flakes" build --no-link --print-out-paths .\#cytrans-web-www -j8)

$SERVER_PATH/bin/server-ng --output-dir . --url-prefix localhost:8080 --static-dir $STATIC_PATH
