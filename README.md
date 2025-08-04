# rns-dns
(WIP) A dns server for the reticulum network stack

# DEV
Run the `tmux.sh` to enter the dev mode. This will automatically place you inside of a `nix develop` flake aka. a `nix-shell`.

> The reason as to why you don't run `nix develop` directly is because tmux tends to bug out with the PS1 prompt of the terminal and with tab completion.

### How to run it

1. start the `rnsd` deamon (usually window 3)
2. start the `client` by running `cargo run --example hello-client`
3. start the `server` by running `cargo run --example echo-server`

