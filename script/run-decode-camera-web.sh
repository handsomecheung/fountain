#!/usr/bin/env bash
set -e

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

cd "$(dirname "${BASH_SOURCE[0]}")/.."

${FOUNTAIN_DOCKER} stop ${FOUNTAIN_CONTAINER_CAMERA_WEB} 2>/dev/null || true
${FOUNTAIN_DOCKER} rm ${FOUNTAIN_CONTAINER_CAMERA_WEB} 2>/dev/null || true
${FOUNTAIN_DOCKER} run --rm -d \
    --name ${FOUNTAIN_CONTAINER_CAMERA_WEB} \
    -p 37250:80 \
    -v "$(pwd)/script/nginx.decode-camera-web.conf:/etc/nginx/nginx.conf" \
    -v "$(pwd)/www:/fountain/www" \
    nginx:latest
