# FLY Bot

## Server Structure

### Chains

* **Ethereum Node**: Located at `/opt/reth/data/reth.ipc`.
* **Base Node**: Located at `/opt/base/data/geth.ipc`.
* **Postgres**: Connection via `/var/run/postgresql/.s.PGSQL.5432`, using the database `fly`.
  Connection url `postgresql://fly@localhost?host=/var/run/postgresql`
* **Lighthouse**: Currently, there is no need to connect to this service.

All services are managed by `systemd`. You can find the respective configuration files in `/etc/systemd/system/`.

All endpoints are group-writable by the `fly` group, and all developers and services are members of this group.

## Deployments

Infrastructure deployment is handled through Ansible playbooks located in the `infra` directory. Currently, only @stas is authorized to perform these deployments.

`fly` deployments involve compiling the bot with the latest changes, copying it to `/opt/fly`, and running it via `systemd`.

## Development

We use a local Rust toolchain for development. Note that it is currently installed only for the `stas` user, and this needs to be addressed to allow broader access.

Development can be conducted locally using `rsync` or `sftp`, or directly on the server.

Connect to Postgres using `psql -U fly` or `export PGUSER=fly` and then `psql`.

Run `cargo clippy` to check for linting errors. Make sure there are not linting errors before pushing final PR or it will fail the CI pipeline.
You can run `cargo make install-hooks` to install it as the pre-push hook.

### To Be Continued...

Further documentation will be added to cover additional aspects of development and deployment processes.




