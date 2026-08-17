#!/usr/bin/env bash
set -euo pipefail

APP_DIR="${APP_DIR:-${HOME}/dike-services}"
APP_NAME="${APP_NAME:-dike-services}"
IMAGE_NAME="${IMAGE_NAME:-dike-services}"
BRANCH="${BRANCH:-main}"
CANDIDATE_NAME="${APP_NAME}-candidate"
ROLLBACK_NAME="${APP_NAME}-rollback"
PORT="${PORT:-4000}"
CANDIDATE_PORT="${CANDIDATE_PORT:-4001}"
HEALTHCHECK_URL="${HEALTHCHECK_URL:-http://127.0.0.1:${PORT}/health}"

cd "$APP_DIR"

git fetch --prune origin
git checkout "$BRANCH"
git pull --ff-only origin "$BRANCH"

REVISION="$(git rev-parse --short=12 HEAD)"
CANDIDATE_IMAGE="${IMAGE_NAME}:${REVISION}"
CANDIDATE_HEALTHCHECK_URL="http://127.0.0.1:${CANDIDATE_PORT}/health"

docker build -t "$CANDIDATE_IMAGE" .
docker rm -f "$CANDIDATE_NAME" >/dev/null 2>&1 || true

cleanup_candidate() {
  docker rm -f "$CANDIDATE_NAME" >/dev/null 2>&1 || true
}
trap cleanup_candidate EXIT

docker run -d \
  --name "$CANDIDATE_NAME" \
  --env-file "$APP_DIR/.env" \
  -p "${CANDIDATE_PORT}:4000" \
  "$CANDIDATE_IMAGE"

candidate_healthy=false
for attempt in $(seq 1 30); do
  if curl --fail --silent --show-error "$CANDIDATE_HEALTHCHECK_URL" >/dev/null; then
    candidate_healthy=true
    break
  fi

  if ! docker ps --format '{{.Names}}' | grep -Fxq "$CANDIDATE_NAME"; then
    echo "Candidate container exited before becoming healthy."
    docker logs --tail=200 "$CANDIDATE_NAME" || true
    exit 1
  fi

  sleep 2
done

if [[ "$candidate_healthy" != true ]]; then
  echo "Candidate container did not become healthy in time."
  docker logs --tail=200 "$CANDIDATE_NAME" || true
  exit 1
fi

docker rm -f "$ROLLBACK_NAME" >/dev/null 2>&1 || true
had_primary=false
if docker container inspect "$APP_NAME" >/dev/null 2>&1; then
  had_primary=true
  docker stop "$APP_NAME" >/dev/null
  docker rename "$APP_NAME" "$ROLLBACK_NAME"
fi

cleanup_candidate

rollback() {
  echo "New primary failed health checks; restoring previous container."
  docker logs --tail=200 "$APP_NAME" || true
  docker rm -f "$APP_NAME" >/dev/null 2>&1 || true

  if [[ "$had_primary" == true ]]; then
    docker rename "$ROLLBACK_NAME" "$APP_NAME"
    docker start "$APP_NAME" >/dev/null
    if curl --retry 15 --retry-delay 2 --retry-connrefused --retry-all-errors \
      --fail --silent --show-error "$HEALTHCHECK_URL" >/dev/null; then
      echo "Previous primary restored."
    else
      echo "Previous primary was restarted but did not become healthy."
      docker logs --tail=200 "$APP_NAME" || true
    fi
  else
    echo "No previous primary was available to restore."
  fi
}

if ! docker run -d \
  --name "$APP_NAME" \
  --restart unless-stopped \
  --env-file "$APP_DIR/.env" \
  -p "${PORT}:4000" \
  "$CANDIDATE_IMAGE"; then
  rollback
  exit 1
fi

if ! curl --retry 15 --retry-delay 2 --retry-connrefused --retry-all-errors \
  --fail --silent --show-error "$HEALTHCHECK_URL" >/dev/null; then
  rollback
  exit 1
fi

docker tag "$CANDIDATE_IMAGE" "${IMAGE_NAME}:latest"
if [[ "$had_primary" == true ]]; then
  docker rm "$ROLLBACK_NAME" >/dev/null
fi

trap - EXIT
echo "Deployed ${CANDIDATE_IMAGE}."
