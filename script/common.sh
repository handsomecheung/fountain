#!/usr/bin/env bash
set -e

DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

export FOUNTAIN_DOCKER="${DOCKER:-docker}"
export FOUNTAIN_IMAGE_DEV=fountain-rust-dev
export FOUNTAIN_CONTAINER_CAMERA_WEB=fountain-decode-camera-web

try_build_dev() {
    if [[ "$(${FOUNTAIN_DOCKER} images -q ${FOUNTAIN_IMAGE_DEV} 2>/dev/null)" == "" ]]; then
        echo "not builder image, building it ..."
        # This build will take significantly longer due to vcpkg compiling OpenCV
        cd "${DIR}/.."
        ${FOUNTAIN_DOCKER} build -t ${FOUNTAIN_IMAGE_DEV} .
        cd -
    fi
}
