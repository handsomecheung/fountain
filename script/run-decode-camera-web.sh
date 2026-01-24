#!/usr/bin/env bash
set -e

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

cd "$(dirname "${BASH_SOURCE[0]}")/.."

${CUBE_DOCKER} stop ${CUBE_CONTAINER_CAMERA_WEB} 2>/dev/null || true
${CUBE_DOCKER} rm ${CUBE_CONTAINER_CAMERA_WEB} 2>/dev/null || true
${CUBE_DOCKER} run --rm -d \
    --name ${CUBE_CONTAINER_CAMERA_WEB} \
    -p 37290:80 \
    -v "$(pwd)/script/nginx.decode-camera-web.conf:/etc/nginx/nginx.conf" \
    -v "$(pwd)/www:/cube/www" \
    nginx:latest
