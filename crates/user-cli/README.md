- [ARPA User CLI](#arpa-user-cli)
  - [Usage](#usage)
    - [REPL Commands](#repl-commands)

# ARPA User CLI

ARPA User CLI is a utilitarian REPL tool to make it easier to interact with smart contracts on ARPA network.

It consists of:

- Randcast: Manage subscriptions and consumer contracts. Query historical requests and fulfillment results.
- Staking: Stake, claim and unstake ARPA tokens. Query staking information and rewards.

## Usage

To print help, use `-- -h`:

```bash
cargo run --bin user-shell -- -h
```

To specify a config file, use `-- -c <config_file>`:

```bash
cargo run --bin user-shell -- -c conf/user_config.yml
```

To set the history file path, use `-- -H <history_file>`:

```bash
cargo run --bin user-shell -- -H user-shell.history
```

### REPL Commands

```text
Commands:
  history   Show command history
  show      Show information of the config file
  randcast  Get views from adapter contract
  stake     Get views from staking contract
  call      Get information from blockchain
  send      *** Be careful this will change on-chain state and cost gas as well as block time***
                Send trxs to on-chain contracts
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```
