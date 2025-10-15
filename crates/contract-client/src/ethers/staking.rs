use alloy::sol;

sol! {
    #[sol(ignore_unlinked)]
    #[sol(rpc)]
    Staking,
    "abi/Staking.json"
}
