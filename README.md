# FLY Bot


## Infra

Infrastructure deployment is handled through Ansible playbooks located in the `infra` directory.
It should fully configure our server given bare Ubuntu instance with `ubuntu` account.
Setup includes:
* Ethereum (prysm/geth)
* Base (node/geth)
* Postgres
* Rust toolchain
* Fly server (`systemd` controlled `/usr/local/bin/fly`)

Infra provisioning is all done at this moment

## Development

Development can be done on a local machine or directly on the server. Local machine is more convenient. On the server
development is mostly useful for benchmarking. It is the different between remote Alchemy RPC (local box) and local IPC RPC (server).

### Principles
* No `panic` in production
* Use `cargo fmt` and `cargo clippy` before committing. This is enforced by CI anyway.
* Follow
### To Be Continued...

Further documentation will be added to cover additional aspects of development and deployment processes.




