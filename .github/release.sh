#!/bin/bash

set -xe # Show output on the logs

# Build the list of `--tag` arguments for a given version.
#
# Arguments:
#   $1 version      e.g. "2.1.3" or "2.1.3-beta.1" (an optional leading "v" is stripped)
#   $2 DOCKER_IMAGE e.g. "lenra/dofigen"
#   $3 prefix       optional tag prefix (e.g. "syntax-"). The "latest" tag becomes the
#                   prefix without its trailing dash (e.g. "syntax-" -> "syntax").
#
# On success the global variable `tag` holds the tag arguments and the function returns 0.
function get_tag {
  version="$1" # Get version tag
  DOCKER_IMAGE="$2"
  prefix="$3" # Optional tag prefix

  # The "latest" tag equivalent: with a prefix "syntax-" it becomes "syntax".
  if [[ -n "${prefix}" ]]; then
    latest="${prefix%-}"
  else
    latest="latest"
  fi

  regex='([0-9]+.[0-9]+.[0-9]+)(-([a-z]+).([0-9]+))?'

  if [[ $version =~ $regex ]]; then
    v="${BASH_REMATCH[1]}"
    channel="${BASH_REMATCH[3]}"
    channel_version="${BASH_REMATCH[4]}"

    tag="--tag ${DOCKER_IMAGE}:${prefix}${version#v}"

    regex='([0-9]+).([0-9]+).([0-9]+)'
    if [[ $v =~ $regex ]]; then
      major=${BASH_REMATCH[1]}
      minor=${BASH_REMATCH[2]}
      patch=${BASH_REMATCH[3]}

      arr_version=( "${major}" "${major}.${minor}" "${major}.${minor}.${patch}" )
      if [[ -n "${channel}" ]]; then
        tag="${tag} --tag ${DOCKER_IMAGE}:${prefix}${channel}"
        for i in "${arr_version[@]}"; do
          tag="${tag} --tag ${DOCKER_IMAGE}:${prefix}${i}-${channel}"
        done
      else
        tag="--tag ${DOCKER_IMAGE}:${latest}"
        for i in "${arr_version[@]}"; do
          tag="${tag} --tag ${DOCKER_IMAGE}:${prefix}${i}"
        done
      fi
      return 0
    else
      echo "Version '$v' didn't pass Regex '$regex'." 1>&2
      return 1
    fi
  else
    echo "Version '$version' didn't pass Regex '$regex'." 1>&2
    return 1
  fi
}

# Only run the build when executed directly (not when sourced by the unit tests).
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -xe # Show output on the logs

  VERSION="$1" # Get version tag
  PREFIX="$2" # Optional tag prefix (e.g. "syntax-")
  DOCKERFILE="${3:-Dockerfile}" # Dockerfile to build

  get_tag "$VERSION" "${DOCKER_IMAGE}" "${PREFIX}"
  exit_code=$?
  if [[ "$exit_code" != "0" ]]; then
    exit $exit_code
  fi

  mkdir -p "~/cache/${DOCKER_IMAGE}-buildcache"

  # build the docker image
  ## Platform argument for arm image : --platform "linux/amd64,linux/arm64,linux/arm" \
  docker buildx build \
    --output type=image,push=true \
    --platform "linux/amd64,linux/arm64" \
    --file "${DOCKERFILE}" \
    ${tag} \
    --cache-from type=local,src=~/cache/${DOCKER_IMAGE}-buildcache \
    --cache-to type=local,dest=~/cache/${DOCKER_IMAGE}-buildcache,mode=max \
    --provenance=true \
    --sbom=true \
    .
fi
