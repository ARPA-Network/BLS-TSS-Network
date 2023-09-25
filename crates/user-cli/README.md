- [Overview](#overview)
- [Dependencies](#dependencies)
- [Usage](#usage)
- [Config](#config)
- [REPL Commands](#repl-commands)
  - [SubCommands](#subcommands)

<h1 align="center">ARPA User CLI</h1>

# Overview

The ARPA User CLI is a REPL tool that makes it easier to interact with smart contracts on the ARPA network.

It consists of:

- Randcast: Manage subscriptions and consumer contracts. Query historical requests and fulfillment results.
- Staking: Stake, claim and unstake ARPA tokens. Query staking information and rewards.

# Dependencies

Install `anvil` and `cast` from [foundry](https://github.com/foundry-rs/foundry#installation), then add them to `PATH`.

(This is Optional, mainly for running `randcast estimate-callback-gas <consumer> <request-sender> <request-signature> <request-params>` command.)

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

# Config

Note: Contract addresses on ETH Mainnet, Sepolia Testnet and Optimism can be found [here](https://docs.arpanetwork.io/randcast/supported-networks-and-parameters).

Configuration items in [`conf/user_config.yml`](conf/user_config.yml) are listed here:

- provider_endpoint: Config http endpoint to interact with chain provider. (example: "http://127.0.0.1:8545")

- chain_id: Config chain id of the network. (example: 31337)

- adapter_address: Config on-chain arpa network Adapter contract address. (example: "0xa513e6e4b8f2a923d98304ec87f64353c4d5c853")

- staking_address: Config on-chain arpa network Staking contract address. (example: "0xcf7ed3acca5a467e9e704c703e8d87f634fb0fc9")

- arpa_address: Config on-chain ARPA token contract address. (example: "0x9fe46736679d2d9a65f0992f2272de9f3c7fa6e0")

- adapter_deployed_block_height: Config the block height when adapter contract is deployed to accelerate the query of events. (example: 100000)

- account: Config the identity of the subscriptions you owned. There are three available account types.

  - example(not recommended): private_key: "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
  - example:
    ```
    keystore:
        password: env
        path: test.keystore
    ```
  - example:

    ```
    hdwallet:
        mnemonic: env
        path: "m/44'/60'/0'/0"
        index: 0
        passphrase: "custom_password"
    ```

    Path and passphrase are optional.

    To protect secrets, several items can be set with environment variables starting with `$`:

  - example:
    - $ARPA_ACCOUNT_PRIVATE_KEY (account, private_key)
    - $ARPA_ACCOUNT_KEYSTORE_PASSWORD (account, keystore, password)
    - $ARPA_HD_ACCOUNT_MNEMONIC (account, hdwallet, mnemonic)

- contract_transaction_retry_descriptor: Config retry strategy for contract transactions. All the time limits are in milliseconds.

  - example:

    ```
      contract_transaction_retry_descriptor:
        base: 2
        factor: 1000
        max_attempts: 3
        use_jitter: true
    ```

- contract_view_retry_descriptor: Config retry strategy for contract views. All the time limits are in milliseconds.

  - example:

    ```
      contract_view_retry_descriptor:
        base: 2
        factor: 500
        max_attempts: 5
        use_jitter: true
    ```

    ```
    We use exponential backoff to retry when an interaction fails. The interval will be an exponent of base multiplied by factor every time. The interval will be reset when the interaction succeeds.

    A jitter is added to the interval to avoid the situation that all the tasks are polling at the same time. It will multiply a random number between 0.5 and 1.0 to the interval.

    contract_transaction_retry_descriptor: (interval sequence without jitter: 2s, 4s, 8s)
    contract_view_retry_descriptor: (interval sequence without jitter: 1s, 2s, 4s, 8s, 16s)
    ```

- relayed_chains: Config chain_id, account, contract addresses, provider endpoint, adapter_deployed_block_height, contract_transaction_retry_descriptor and contract_view_retry_descriptor for all relayed chains we support.

  - example:

    ```
      relayed_chains:
        - chain_id: 901
          provider_endpoint: "http://127.0.0.1:9545"
          adapter_address: "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707"
          adapter_deployed_block_height: 0
          arpa_address: "0x9fe46736679d2d9a65f0992f2272de9f3c7fa6e0"
          account:
            private_key: $OP_ACCOUNT_PRIVATE_KEY
          contract_transaction_retry_descriptor:
            base: 2
            factor: 1000
            max_attempts: 3
            use_jitter: true
          contract_view_retry_descriptor:
            base: 2
            factor: 500
            max_attempts: 5
            use_jitter: true
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
  block              Get block information [aliases: b]
  current-gas-price  Get current gas price [aliases: cgp]
  trx-receipt        Get transaction receipt [aliases: tr]
  balance-of-eth     Get balance of eth [aliases: boe]
  balance-of-arpa    Get balance of arpa [aliases: boa]
  help               Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```text
*** Be careful this will change on-chain state and cost gas as well as block time***
Send trxs to on-chain contracts

Usage: send [COMMAND]

Commands:
  approve-arpa-to-staking     Approve arpa to staking contract [aliases: aats]
  stake                       Stake arpa to staking contract [aliases: s]
  unstake                     Unstake(then freeze) arpa from staking contract and claim delegation rewards instantly after exit [aliases: u]
  claim-frozen-principal      Claim frozen principal from staking after unstake [aliases: cfp]
  claim                       Claim rewards as well as frozen principal(if any) from staking [aliases: c]
  claim-reward                Claim rewards from staking [aliases: cr]
  create-subscription         Create a new subscription as owner [aliases: cs]
  add-consumer                Add consumer contract to subscription [aliases: ac]
  fund-subscription           Fund subscription with ETH [aliases: fs]
  set-referral                Set referral subscription id for your subscription to get referral rewards [aliases: sr]
  cancel-subscription         Cancel subscription and redeem ETH left to receiver address [aliases: ccs]
  remove-consumer             Remove consumer contract from subscription [aliases: rc]
  set-callback-gas-config     Set callback gas config for consumer contract [aliases: scgc]
  set-request-confirmations   Set request confirmations for consumer contract [aliases: src]
  help                        Print this message or the help of the given subcommand(s)

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
  estimate-callback-gas       Estimate callback gas for any consumer contract extends GeneralRandcastConsumerBase under the current circumstances. This can be used before the first request to estimate how much eth is needed for subscription funding, or at any time to compare gas cost with the estimated one to adjust the callback gas config in the consumer contract. This also can be used as a dry run to see if the callback function in consumer contract reverts due to business logic or gas limit. An error will be returned if callback in the consumer contract reverts. [aliases: ecg]
  estimate-payment-amount     Estimate the amount of gas used for a fulfillment of randomness in 3 times of current gas price, for calculating how much eth is needed for subscription funding [aliases: epa]
  callback-gas-limit          Get callback gas limit of consumer contract [aliases: cgl]
  callback-max-gas-fee        Get callback max gas fee of consumer contract. 0 means auto-estimating CallbackMaxGasFee as 3 times tx.gasprice of the request call, also user can set it manually by calling set-callback-gas-config [aliases: cmgf]
  nonces                      Get nonce(counting from 1, as there was no request) for a specific subscription id and consumer address [aliases: n]
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
