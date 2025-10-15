use alloy::sol;

sol! {
    #[sol(ignore_unlinked)]
    #[sol(rpc)]
    ServiceManager,
    "abi/ServiceManager.json"
}
