#!/usr/bin/env bash
set -Eeuo pipefail
umask 077
[[ "$BASE_PATH_INPUT" == /opt/* && "$BASE_PATH_INPUT" != *..* ]]
[[ "$SERVICE_PATH_INPUT" == /opt/* && "$SERVICE_PATH_INPUT" != *..* ]]
[[ "$COMPOSE_PROJECT_NAME_INPUT" == bells-mainnet ]]
export COMPOSE_PROJECT_NAME="$COMPOSE_PROJECT_NAME_INPUT"
CONTAINER=bells-mainnet-ord
exec 9>"$SERVICE_PATH_INPUT/.deploy.lock"
flock -w 5 9
[ ! -d "$SERVICE_PATH_INPUT/.deploy-transaction" ] || { echo 'Resolve pending deployment before resetting Ord.'; exit 1; }
python3 - <<'PY'
import json, os, subprocess
from pathlib import Path
p = Path('/home/ord/ord_db')
assert str(p.resolve()) == str(p) and p.is_dir(), 'Invalid state directory'
assert p.stat().st_dev == Path('/home').stat().st_dev != Path('/opt').stat().st_dev, 'Separate /home disk required'
assert subprocess.check_output(['findmnt', '-n', '-o', 'TARGET', '-T', str(p)], text=True).strip() == '/home'
all_containers = subprocess.check_output(['docker', 'ps', '-aq'], text=True).split()
containers = json.loads(subprocess.check_output(['docker', 'inspect', *all_containers]))
for c in containers:
    for m in c['Mounts']:
        if m['Source'].startswith(str(p)):
            assert c['Name'] == '/bells-mainnet-ord', 'Another container uses Ord state'
c = next(c for c in containers if c['Name'] == '/bells-mainnet-ord')
assert c['Config'].get('Healthcheck'), 'Deploy the shutdown-safe image before resetting'
assert any(m['Source'] == str(p) and m['Destination'] == '/app/ord_db' and m['Type'] == 'bind' for m in c['Mounts'])
assert not (p / 'index.redb').is_symlink(), 'Index must not be a symlink'
PY
if [ "$(docker inspect -f '{{.State.Running}}' "$CONTAINER")" = true ]; then
  docker update --restart=no "$CONTAINER" >/dev/null
  docker kill --signal=SIGTERM "$CONTAINER" >/dev/null
  deadline=$((SECONDS + 900))
  while [ "$(docker inspect -f '{{.State.Running}}' "$CONTAINER")" = true ]; do
    [ "$SECONDS" -lt "$deadline" ] || { echo 'Shutdown still in progress; state retained.'; exit 1; }
    sleep 2
  done
  [ "$(docker inspect -f '{{.State.ExitCode}} {{.State.OOMKilled}}' "$CONTAINER")" = '0 false' ] || { echo 'Unclean shutdown; state retained for review.'; exit 1; }
fi
# Remove only the explicitly named index; retain the mount, directory and config.
rm -f -- /home/ord/ord_db/index.redb
chown 1001:1001 /home/ord/ord_db
docker compose -f "$BASE_PATH_INPUT/docker-compose.yml" up -d --no-deps --force-recreate "$CONTAINER"
deadline=$((SECONDS + 300))
while [ "$SECONDS" -lt "$deadline" ]; do
  state=$(docker inspect -f '{{.State.Status}} {{.State.Health.Status}}' "$CONTAINER")
  [ "$state" != 'running healthy' ] || { echo 'Ord index reset; serving reads while initial indexing continues on /home.'; exit 0; }
  case "$state" in exited*|dead*|'running unhealthy') echo 'Ord failed readiness after reset.'; exit 1 ;; esac
  sleep 3
done
echo 'Readiness timed out after reset; inspect Ord without deleting more state.'
exit 1
