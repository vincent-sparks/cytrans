#!/bin/sh

CYTRANS_WEB_ROOT=$(dirname $0)

SERVER_PATH=$(nix --extra-experimental-features "nix-command flakes" build --no-link --print-out-paths $CYTRANS_WEB_ROOT/..\#cytrans-web-server -j$(nproc)) || exit
if [[ "$1" == "--compressed" ]]; then 
    shift
    STATIC_PATH=$(nix --extra-experimental-features "nix-command flakes" build --no-link --print-out-paths $CYTRANS_WEB_ROOT/..\#cytrans-web-www-compressed -j$(nproc)) || exit
else
    CLIENT_PATH=$(nix --extra-experimental-features "nix-command flakes" build --no-link --print-out-paths $CYTRANS_WEB_ROOT/..\#cytrans-web-client -j$(nproc)) || exit
    rm $CYTRANS_WEB_ROOT/www/client
    ln -s $CLIENT_PATH/client $CYTRANS_WEB_ROOT/www/client
    STATIC_PATH=$CYTRANS_WEB_ROOT/www
fi

RUST_LOG=debug $SERVER_PATH/bin/server-ng --output-dir . --url-prefix localhost:8080 --static-dir $STATIC_PATH $@
