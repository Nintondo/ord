#!/usr/bin/env bash
set -Eeuo pipefail

for value in "$SERVICE_NAME" "${COIN:-}" "${NETWORK:-}" "${SERVICE_ENVIRONMENT:-}"; do
  [[ -z "$value" || "$value" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo 'Invalid service or environment segment'; exit 1; }
done
if [ "${MAINTENANCE_ONLY:-false}" != true ]; then
  [[ "$SERVICE_TAG" =~ ^[a-f0-9]{40}$ ]] || { echo 'Deploy requires a full immutable commit SHA'; exit 1; }
fi
for value in "$DOCKER_COMPOSE_FILE" "${SERVICE_COMPOSE_FILE:-}" "${COMPOSE_BASE_FILE:-}" "${CONFIG_TARGET_FILE:-}" "${COMPOSE_UPDATED_FILE:-}" "${CONFIG_UPDATED_FILE:-}" "${ENV_UPDATED_FILE:-}" "${CONF_DIR:-}" "${LIST_UPDATED_FILE:-}"; do
  [[ "$value" != /* && "$value" != *..* ]] || { echo 'Service file paths must be relative and stay inside the service'; exit 1; }
done

BASE_PATH="/opt/${COIN}-${NETWORK}"
SERVICE_PATH="${BASE_PATH}/services/${SERVICE_NAME}"
IMAGE_NAME="${IMAGE_NAME_OVERRIDE:-$SERVICE_NAME}"
IMAGE="${CI_REGISTRY}/${CI_REGISTRY_REPO}/${IMAGE_NAME}:${SERVICE_TAG}"
CONTAINER="${COIN}-${NETWORK}-${SERVICE_NAME}"
COMPOSE_SERVICE="${COMPOSE_SERVICE_NAME_OVERRIDE:-${COMPOSE_SERVICE:-$CONTAINER}}"
# Explicit deployment paths let SCP and Compose use the same canonical layout.
if [ -n "${BASE_PATH_INPUT:-}" ]; then BASE_PATH="$BASE_PATH_INPUT"; fi
if [ -n "${SERVICE_PATH_INPUT:-}" ]; then SERVICE_PATH="$SERVICE_PATH_INPUT"; fi
for deployment_path in "$BASE_PATH" "$SERVICE_PATH"; do
  [[ "$deployment_path" == /opt/* && "$deployment_path" != *..* ]] || { echo 'Deployment paths must stay inside /opt'; exit 1; }
done
if [ -n "${COMPOSE_PROJECT_NAME_INPUT:-}" ]; then
  [[ "$COMPOSE_PROJECT_NAME_INPUT" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo 'Invalid Compose project name'; exit 1; }
  export COMPOSE_PROJECT_NAME="$COMPOSE_PROJECT_NAME_INPUT"
fi
TARGET_COMPOSE="$SERVICE_PATH/${DOCKER_COMPOSE_FILE}"

compose() {
local command=(docker compose)
if [ -n "${COMPOSE_COMMAND_TIMEOUT:-}" ]; then command=(timeout "$COMPOSE_COMMAND_TIMEOUT" docker compose); fi
"${command[@]}" -f "$BASE_PATH/docker-compose.yml" "$@"
}

snapshot_files() {
backup_file "$TARGET_COMPOSE" || return 1
if [ -n "$ENV_UPDATED_FILE" ]; then backup_file "$SERVICE_PATH/.env" || return 1; fi
}

apply_files() {
install -m 600 "$SERVICE_PATH/ord.yml.updated" "$TARGET_COMPOSE"
if [ -n "$ENV_UPDATED_FILE" ]; then mv "$SERVICE_PATH/$ENV_UPDATED_FILE" "$SERVICE_PATH/.env"; chmod 600 "$SERVICE_PATH/.env"; fi
}

run_migrations() {
:
}

verify_configured_image() {
local configured
configured=$(compose config --format json | python3 -c 'import json,sys; print(json.load(sys.stdin)["services"][sys.argv[1]]["image"])' "$COMPOSE_SERVICE")
[ "$configured" = "$IMAGE" ]
}

verify_state_location() {
python3 - <<'PY_STATE'
import os, subprocess
from pathlib import Path
p = Path('/home/ord/ord_db')
assert str(p.resolve()) == str(p), 'Ord state path must not be a symlink'
assert p.is_dir(), 'Ord state directory is missing'
assert p.stat().st_dev == Path('/home').stat().st_dev, 'State must be on /home filesystem'
assert p.stat().st_dev != Path('/opt').stat().st_dev, 'Separate /home filesystem is required'
assert subprocess.check_output(['findmnt', '-n', '-o', 'TARGET', '-T', str(p)], text=True).strip() == '/home', 'Unexpected state mount'
PY_STATE
}

verify_runtime() {
verify_state_location
python3 - "$CONTAINER" "$IMAGE" <<'PY_RUNTIME'
import json, subprocess, sys
c = json.loads(subprocess.check_output(['docker', 'inspect', sys.argv[1]]))[0]
assert c['Config']['Image'] == sys.argv[2], 'Wrong image'
mounts = [m for m in c['Mounts'] if m['Destination'] == '/app/ord_db']
assert len(mounts) == 1 and mounts[0]['Type'] == 'bind' and mounts[0]['Source'] == '/home/ord/ord_db' and mounts[0]['RW'], 'Wrong Ord data mount'
env = dict(v.split('=', 1) for v in c['Config']['Env'])
assert env.get('ORD_DATA_DIR') == '/app/ord_db', 'Wrong Ord data directory'
assert c['Config'].get('StopSignal') == 'SIGTERM', 'Wrong stop signal'
assert c['Config'].get('Labels', {}).get('org.opencontainers.image.revision') == sys.argv[2].rsplit(':', 1)[1], 'Image revision mismatch'
PY_RUNTIME
}

stop_cleanly() {
  local running deadline state containers
  if ! running=$(docker inspect -f '{{.State.Running}}' "$CONTAINER" 2>/dev/null); then
    # Compose may fail after removing the old container. If Docker is available
    # and the name is absent, rollback can safely recreate it from the snapshot.
    containers=$(docker ps -aq --filter "name=^/${CONTAINER}$") || return 1
    [ -z "$containers" ] || return 1
    return 0
  fi
  if [ "$running" != true ]; then
    if [ -e /home/ord/ord_db/index.redb ]; then
      state=$(docker inspect -f '{{.State.ExitCode}} {{.State.OOMKilled}}' "$CONTAINER") || return 1
      if [ "$state" != '0 false' ]; then
        touch "$TRANSACTION/shutdown-blocked"
        echo 'Existing index follows an unclean exit; inspect it before restarting.' >&2
        return 1
      fi
    fi
    return 0
  fi
  # Disable automatic restart while signalling; never let Compose escalate to KILL.
  docker update --restart=no "$CONTAINER" >/dev/null || return 1
  docker kill --signal=SIGTERM "$CONTAINER" >/dev/null || return 1
  deadline=$((SECONDS + ${STOP_TIMEOUT_SECONDS:-900}))
  while [ "$SECONDS" -lt "$deadline" ]; do
    running=$(docker inspect -f '{{.State.Running}}' "$CONTAINER") || return 1
    if [ "$running" = false ]; then
      state=$(docker inspect -f '{{.State.ExitCode}} {{.State.OOMKilled}}' "$CONTAINER") || return 1
      if [ "$state" = '0 false' ]; then return 0; fi
      touch "$TRANSACTION/shutdown-blocked"
      echo 'Ord exited uncleanly; deployment stopped without recreating or removing state.' >&2
      return 1
    fi
    sleep 2
  done
  touch "$TRANSACTION/shutdown-blocked"
  echo 'Ord is still shutting down; no SIGKILL sent. Inspect retained transaction before retry.' >&2
  return 1
}

legacy_probe() {
docker exec -i "$CONTAINER" sh <<'LEGACY_PROBE'
awk '$2 ~ /:0D05$/ && $4 == "0A" {found=1} END {exit !found}' /proc/net/tcp /proc/net/tcp6
LEGACY_PROBE
}


# One persistent transaction spans remote readiness and the CI public probe.
# The directory also blocks a second deployment while recovery is unresolved.
umask 077
[[ "$DEPLOYMENT_ID" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo 'Invalid deployment ID'; exit 1; }
[[ "$HEALTH_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || { echo 'Invalid health timeout'; exit 1; }
test -d "$SERVICE_PATH"
cd "$BASE_PATH"
exec 9>"$SERVICE_PATH/.deploy.lock"
flock -w 5 9
TRANSACTION="$SERVICE_PATH/.deploy-transaction"

backup_file() {
  local target="$1" index
  [[ "$target" = /* ]] || { echo 'Backup target must be absolute'; return 1; }
  index=$(find "$TRANSACTION" -maxdepth 1 -name 'target-*' | wc -l) || return 1
  printf '%s' "$target" > "$TRANSACTION/target-$index" || return 1
  if [ -e "$target" ]; then
    cp -p "$target" "$TRANSACTION/file-$index" || return 1
  else
    touch "$TRANSACTION/absent-$index" || return 1
  fi
}

wait_ready() {
  local mode="$1" state='' deadline=$((SECONDS + HEALTH_TIMEOUT_SECONDS)) stable=0
  while [ "$SECONDS" -lt "$deadline" ]; do
    state=$(docker inspect -f '{{.State.Status}} {{if .State.Health}}{{.State.Health.Status}}{{else}}missing{{end}}' "$CONTAINER") || return 1
    case "$state" in
      'running healthy') return 0 ;;
      'running missing')
        if [ "$mode" = rollback ]; then
          # Older images do not yet expose /readyz or Docker HEALTHCHECK.
          # Check their existing RPC (nodes) or listening socket (applications).
          if legacy_probe; then
            stable=$((stable + 1))
            [ "$stable" -lt 3 ] || return 0
          else
            stable=0
          fi
        else
          echo 'New image must provide a Docker healthcheck'; return 1
        fi ;;
      'running unhealthy'|exited*|dead*) echo "Readiness failed: $state"; return 1 ;;
    esac
    sleep 3
  done
  echo "Readiness timed out: $state"
  return 1
}

restore_transaction() {
  local target marker index
  [ ! -f "$TRANSACTION/shutdown-blocked" ] || { echo "Manual shutdown recovery required at $TRANSACTION" >&2; return 1; }
  stop_cleanly || return 1
  # Do not swallow errors or delete the only recovery copy on failure.
  for marker in "$TRANSACTION"/target-*; do
    [ -f "$marker" ] || continue
    target=$(cat "$marker")
    index=${marker##*/target-}
    if [ -f "$TRANSACTION/absent-$index" ]; then
      rm -f "$target" || return 1
    else
      cp -p "$TRANSACTION/file-$index" "$target" || return 1
    fi
  done
  [ ! -f "$TRANSACTION/shutdown-blocked" ] || { echo 'Shutdown requires manual review; rollback will not force-stop Ord.' >&2; return 1; }
  compose config --quiet || return 1
  if [ "$(cat "$TRANSACTION/previous-running")" = true ]; then
    compose up -d --no-deps --force-recreate "$COMPOSE_SERVICE" || return 1
  else
    compose create --no-deps --force-recreate "$COMPOSE_SERVICE" || return 1
  fi
  [ "$(docker inspect -f '{{.Config.Image}}' "$CONTAINER")" = "$(cat "$TRANSACTION/previous-image")" ] || return 1
  if [ "$(cat "$TRANSACTION/previous-running")" = true ]; then wait_ready rollback || return 1; fi
  if [ "$RELOAD_NGINX" = true ]; then
    docker exec nginx nginx -t || return 1
    docker exec nginx nginx -s reload || return 1
  fi
  touch "$TRANSACTION/restored"
  echo 'Previous image and service files restored; backups retained for review.'
}

owns_transaction() {
  [ -f "$TRANSACTION/id" ] && [ "$(cat "$TRANSACTION/id")" = "$DEPLOYMENT_ID" ]
}

case "$DEPLOY_PHASE" in
  rollback)
    if ! owns_transaction; then
      echo 'No transaction belonging to this run; nothing to roll back.'
      exit 0
    fi
    [ ! -f "$TRANSACTION/restored" ] || exit 0
    if ! restore_transaction; then
      echo "ROLLBACK FAILED; recovery files retained at $TRANSACTION" >&2
      exit 1
    fi
    exit 0 ;;
  commit)
    owns_transaction || { echo 'Deployment transaction is missing'; exit 1; }
    [ ! -f "$TRANSACTION/restored" ] || { echo 'Cannot commit a restored deployment'; exit 1; }
    [ "$(docker inspect -f '{{.Config.Image}}' "$CONTAINER")" = "$IMAGE" ]
    wait_ready "${READINESS_MODE:-deploy}"
    verify_runtime
    rm -rf "$TRANSACTION"
    echo 'Deployment committed after all readiness checks.'
    exit 0 ;;
  deploy) ;;
  *) echo 'Unknown deployment phase'; exit 1 ;;
esac

if [ -d "$TRANSACTION" ]; then
  # A successfully restored prior run can be archived on the next deployment.
  if [ -f "$TRANSACTION/restored" ]; then
    mv "$TRANSACTION" "$SERVICE_PATH/.deploy-restored-$(cat "$TRANSACTION/id")"
  else
    echo "Unresolved deployment at $TRANSACTION; recover it before deploying." >&2
    exit 1
  fi
fi
verify_state_location
PREVIOUS_RUNNING=$(docker inspect -f '{{.State.Running}}' "$CONTAINER")
PREVIOUS_IMAGE=$(docker inspect -f '{{.Config.Image}}' "$CONTAINER")
[ -n "$PREVIOUS_IMAGE" ]
mkdir -m 700 "$TRANSACTION"
printf '%s' "$DEPLOYMENT_ID" > "$TRANSACTION/id"
printf '%s' "$PREVIOUS_IMAGE" > "$TRANSACTION/previous-image"
printf '%s' "$PREVIOUS_RUNNING" > "$TRANSACTION/previous-running"
# A failed snapshot leaves the running service untouched.
if ! snapshot_files; then
  rm -rf "$TRANSACTION"
  echo 'Cannot snapshot service files; deployment has not started.' >&2
  exit 1
fi

deployment_exit() {
  local status=$?
  trap - EXIT INT TERM
  if [ "$status" -ne 0 ]; then
    echo 'Deployment failed; restoring previous service files and image.'
    if ! restore_transaction; then
      echo "ROLLBACK FAILED; recovery files retained at $TRANSACTION" >&2
    fi
  fi
  exit "$status"
}
trap deployment_exit EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
docker pull "$IMAGE"
stop_cleanly
apply_files
compose config --quiet
verify_configured_image
run_migrations
compose up -d --no-deps --force-recreate "$COMPOSE_SERVICE"
[ "$(docker inspect -f '{{.Config.Image}}' "$CONTAINER")" = "$IMAGE" ]
wait_ready "${READINESS_MODE:-deploy}"
verify_runtime
if [ "$RELOAD_NGINX" = true ]; then
  docker exec nginx nginx -t
  docker exec nginx nginx -s reload
fi
trap - EXIT INT TERM
echo "Remote readiness passed; transaction remains pending at $TRANSACTION"
