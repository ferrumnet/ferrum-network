#!/bin/bash
if [ "$#" -ne 2 ]; then
  echo "Usage: ${0} IMAGE INSTANCE" >&2
  exit 1
fi
if [ -z "${1}" ]; then
    echo "Error: the IMAGE argument is empty." >&2
    exit 1
fi
if [ -z "${2}" ]; then
    echo "Error: the INSTANCE argument is empty." >&2
    exit 1
fi
docker pull "${1}"
IMAGE_DIGEST=$(docker image inspect --format='{{json .Id}}' "${1}" | tr -d '"')
INSTANCE_DIGEST=$(docker inspect --format='{{json .Image}}' "${2}" | tr -d '"')
test "${IMAGE_DIGEST}" = "${INSTANCE_DIGEST}"
docker-compose pull
docker-compose up -d
exit $?