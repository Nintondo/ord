# Ord operations

Ord runs only in `bells-mainnet`, on `162.55.243.23`. The canonical Compose
project directory is `/opt/nintondo/bells/mainnet` (compatibility alias for the
existing `/opt/bells-mainnet`); the Compose project name remains `bells-mainnet`.
Service configuration is in `services/ord`. Ord talks to the primary Bells node
via `http://bellscoin-mainnet:19918` on the existing shared Docker network.

## State and indexing

The only Ord index is `/home/ord/ord_db/index.redb`, bind-mounted at
`/app/ord_db`. `/home` is a separate filesystem (`/dev/md3`); it must be mounted
before Ord starts. Root and /home are separate RAID1 partitions on the same
pair of physical NVMe drives; moving state to /home does not isolate physical I/O.
Compose refuses to create a missing bind source. Deployment
and maintenance scripts additionally check the actual filesystem and reject a
symlinked state directory. App UID/GID are `1001:1001`.

`ORD_DATA_DIR=/app/ord_db`; `ORD_INDEX_CACHE_SIZE=4294967296` limits the redb
cache to 4 GiB instead of implicitly using a quarter of host RAM. This is a
cache budget, not a limit on total process memory. Commit interval and sats,
runes and address indexing retain their existing settings. Index data is never
removed or restored by deploy/rollback. Rollback applies only the image and
configuration, and preserves whether the old service was stopped.

`/healthz` responds while HTTP is running and shutdown has not begun. `/readyz`
requires a readable index and a live indexer with no unrecoverable reorg.
Both probes are read-only. Readiness does **not** mean initial sync reached the
chain tip: compare `/blockcount` with node height to track sync. Docker probes
`/readyz` every 15 seconds with a 60-second startup period.

## Shutdown and restart

SIGTERM/SIGINT set the shutdown flag and drain HTTP for up to 30 seconds. The
main thread waits for the indexer to finish the current block, commit the
pending batch and close redb. Repeated signals are idempotent. Fetching the next
block is cancellable while waiting for RPC retries. Transaction failures roll
back through redb; no on-disk schema or redb version is changed in this patch.

Compose uses `stop_signal: SIGTERM` and `stop_grace_period: 15m`. Docker can
still send SIGKILL after a Compose stop timeout; a longer grace period alone is
not a correctness guarantee. For a routine restart on the host use:

```sh
bash /opt/nintondo/bells/mainnet/services/ord/scripts/restart_ord.sh
```

The script disables automatic restart, sends SIGTERM, waits for exit code 0
without OOM, then starts only Ord and waits for readiness. It shares the deploy
lock. A timeout leaves the container and state in place, with automatic restart
disabled, for investigation; no forced kill is sent. Do not use `docker rm -f`,
`docker kill` without an explicit SIGTERM, or retry a blocked shutdown blindly.
OOM, SIGKILL, host power loss and disk failure cannot be handled by a signal
handler. redb has crash recovery, but it does not replace backups or make such
failures impossible. No full index backup is required for this authorized reset.

## CI/CD and recovery

All jobs use self-hosted runners. Pushes to main build and publish a tested image
tagged with the full commit SHA; only manual `Deploy service` with
`confirm_mainnet=DEPLOY_MAINNET` deploys to mainnet. The selected SHA supplies
both the image and configuration. Do not deploy a historical SHA lacking this
shutdown contract. Build uses the locked dependencies; the Linux subprocess
regression tests run before release compilation. Their mock blocks use valid
Bells subsidies.

GitHub environment `bells-mainnet` owns `BASE_PATH`, `SERVICE_DIR_1`,
`COMPOSE_PROJECT_NAME`, indexing/cache/RPC/data settings and RPC/SSH credentials.
Registry credentials are the existing CI account; host pulls use the existing
read-only registry account. Rendered `.env` and recovery snapshots are private.

Deployment pulls the image, obtains the service lock and keeps a transaction in
`services/ord/.deploy-transaction`. It cleanly stops Ord before changing its
configuration or recreating the container (`--no-deps`), verifies the image,
mount and readiness, then commits. Failure restores the previous configuration
and image while retaining the same index. If clean shutdown fails, automatic
rollback must not force-stop the service; snapshots are retained and the job
fails. Inspect `id`, `previous-image`, `previous-running`, `shutdown-blocked`
and `target-*` files on the host before recovery. Do not delete the transaction
merely to unblock another deployment. No other service, node or indexer is
recreated by these workflows. No nginx reload is needed for an Ord-only update.

## Explicit index reset

`Wipe service` is a separate manual maintenance operation guarded by
`confirm_wipe=WIPE_MAINNET` and the same concurrency group/lock as deploy. It
requires the shutdown-safe image, stops Ord cleanly, verifies that no other
container uses the state directory, and removes **only** `index.redb`. It
preserves the directory, filesystem, Compose files, credentials and all node /
Electrs / token data. It then starts only Ord. A reset causes a full reindex;
index-dependent APIs can return incomplete results until sync reaches the tip.

References: [Docker stop semantics](https://docs.docker.com/reference/cli/docker/container/stop/),
[redb durability and recovery design](https://github.com/cberner/redb/blob/master/docs/design.md).
