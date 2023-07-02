- [ARPA User CLI](#arpa-user-cli)
- [Dependencies](#dependencies)
- [Usage](#usage)
- [REPL Commands](#repl-commands)
  - [SubCommands](#subcommands)

# ARPA User CLI

ARPA User CLI is a utilitarian REPL tool to make it easier to interact with smart contracts on ARPA network.

It consists of:

- Randcast: Manage subscriptions and consumer contracts. Query historical requests and fulfillment results.
- Staking: Stake, claim and unstake ARPA tokens. Query staking information and rewards.

# Dependencies

Install `anvil` and `cast` from [foundry](https://github.com/foundry-rs/foundry#installation), then add them to `PATH` for `randcast estimate-callback-gas <consumer> <request-sender> <request-signature> <request-params>` command.

# Usage

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

# REPL Commands

```text
Commands:
  history   Show command history
  show      Show information of the config file
  randcast  Get views and events from adapter contract
  stake     Get views and events from staking contract
  call      Get information from blockchain
  send      *** Be careful this will change on-chain state and cost gas as well as block time***
                Send trxs to on-chain contracts
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## SubCommands

```text
Show information of the config file

Usage: show [COMMAND]

Commands:
  address  Show address of the wallet [aliases: a]
  config   Print config [aliases: c]
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```text
Get information from blockchain

Usage: call [COMMAND]

Commands:
  block            Get block information [aliases: b]
  trx-receipt      Get transaction receipt [aliases: tr]
  balance-of-eth   Get balance of eth [aliases: boe]
  balance-of-arpa  Get balance of arpa [aliases: boa]
  help             Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```text
*** Be careful this will change on-chain state and cost gas as well as block time***
Send trxs to on-chain contracts

Usage: send [COMMAND]

Commands:
  approve-arpa-to-staking  Approve arpa to staking contract [aliases: aats]
  stake                    Stake arpa to staking contract [aliases: s]
  unstake                  Unstake(then freeze) arpa from staking contract and claim delegation rewards instantly after exit [aliases: u]
  claim-frozen-principal   Claim frozen principal from staking after unstake [aliases: cfp]
  claim                    Claim rewards as well as frozen principal(if any) from staking [aliases: c]
  claim-reward             Claim rewards from staking [aliases: cr]
  create-subscription      Create a new subscription as owner [aliases: cs]
  add-consumer             Add consumer contract to subscription [aliases: ac]
  fund-subscription        Fund subscription with ETH [aliases: fs]
  set-referral             Set referral subscription id for your subscription to get referral rewards [aliases: sr]
  cancel-subscription      Cancel subscription and redeem ETH left to receiver address [aliases: ccs]
  remove-consumer          Remove consumer contract from subscription [aliases: rc]
  help                     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```text
Get views and events from adapter contract

Usage: randcast [COMMAND]

Commands:
  subscription                Get subscription by subscription id [aliases: s]
  my-subscriptions            Get my subscriptions [aliases: mss]
  consumers                   Get consumer contracts by subscription id [aliases: cs]
  requests                    Get requests by subscription id, filter by consumer address, pending/ success/ failed [aliases: rs]
  estimate-callback-gas       Estimate callback gas for any consumer contract extends GeneralRandcastConsumerBase before the first request. This also can be used as a dry run for the first request. An error will be returned if callback in the consumer contract reverts. [aliases: ecg]
  estimate-payment-amount     Estimate the amount of gas used for a fulfillment of randomness in 3 times of current gas price, for calculating how much eth is needed for subscription funding [aliases: epa]
  last-randomness             Get last randomness [aliases: lr]
  pending-request-commitment  Get pending commitment by request id [aliases: prc]
  adapter-config              Get adapter config [aliases: ac]
  flat-fee-config             Get flat fee info about fee tiers, if global flat fee promotion is enabled and flat fee promotion global percentage and duration [aliases: ffc]
  referral-config             Get info about if referral activity is enabled and free request count for referrer and referee [aliases: rcfg]
  fee-tier                    Get fee tier based on the request count [aliases: ft]
  last-assigned-group-index   Get last assigned group index in randomness generation [aliases: lagi]
  randomness-count            Get randomness count [aliases: rc]
  cumulative-data             Get cumulative data(FlatFee, CommitterReward and PartialSignatureReward) of randomness generation [aliases: cd]
  help                        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```text
Get views and events from staking contract

Usage: stake [COMMAND]

Commands:
  stake                          Get staked arpa amount [aliases: s]
  base-reward                    Get amount of base rewards earned in ARPA wei [aliases: br]
  delegation-reward              Get amount of delegation rewards earned by an operator in ARPA wei [aliases: dr]
  total-delegated-amount         Get total delegated amount, calculated by dividing the total community staker staked amount by the delegation rate, i.e. totalDelegatedAmount = pool.totalCommunityStakedAmount / delegationRateDenominator [aliases: tda]
  delegates-count                Delegates count increases after an operator is added to the list of operators and stakes the required amount. [aliases: dc]
  community-stakersCount         Count all community stakers that have a staking balance greater than 0 [aliases: cs]
  getTotalStakedAmount           Total amount staked by community stakers and operators in ARPA wei [aliases: tsa]
  total-community-staked-amount  Total amount staked by community stakers in ARPA wei [aliases: tcsa]
  total-frozen-amount            The sum of frozen operator principals that have not been withdrawn from the staking pool in ARPA wei. [aliases: tfa]
  delegation-rate-denominator    Get current delegation rate [aliases: drd]
  reward-rate                    Get current reward rate, expressed in arpa weis per second [aliases: rr]
  reward-apy                     Get current reward APY, expressed in percentage [aliases: ra]
  reward-timestamps              Get reward initialization timestamp and reward expiry timestamp [aliases: rt]
  operator-limit                 Get amount that should be staked by an operator [aliases: ol]
  community-staker-limits        Get minimum amount and maximum amount that can be staked by a community staker [aliases: csl]
  max-pool-size                  Get current maximum staking pool size [aliases: mps]
  frozen-principal               Get frozen principal and unfreeze time [aliases: fp]
  help                           Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```
