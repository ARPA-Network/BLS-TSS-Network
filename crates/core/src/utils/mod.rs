use alloy::{
    eips::eip1559::Eip1559Estimation,
    hex,
    primitives::{utils::keccak256, Address, FixedBytes, U256},
};
use chrono::Local;
use log::info;

/// The threshold max change/difference (in %) at which we will ignore the fee history values
/// under it.
pub const EIP1559_FEE_ESTIMATION_THRESHOLD_MAX_CHANGE: i64 = 200;
pub const OP_MAINNET_CHAIN_ID: u64 = 10;
pub const OP_GOERLI_TESTNET_CHAIN_ID: u64 = 420;
pub const OP_SEPOLIA_TESTNET_CHAIN_ID: u64 = 11155420;
pub const OP_DEVNET_CHAIN_ID: u64 = 901;
pub const BASE_MAINNET_CHAIN_ID: u64 = 8453;
pub const BASE_GOERLI_TESTNET_CHAIN_ID: u64 = 84531;
pub const BASE_SEPOLIA_TESTNET_CHAIN_ID: u64 = 84532;
pub const REDSTONE_HOLESKY_TESTNET_CHAIN_ID: u64 = 17001;
pub const REDSTONE_MAINNET_CHAIN_ID: u64 = 690;
pub const REDSTONE_GARNET_TESTNET_CHAIN_ID: u64 = 17069;
pub const LOOT_MAINNET_CHAIN_ID: u64 = 5151706;
pub const LOOT_TESTNET_CHAIN_ID: u64 = 9088912;
pub const TAIKO_HEKLA_TESTNET_CHAIN_ID: u64 = 167009;
pub const TAIKO_MAINNET_CHAIN_ID: u64 = 167000;
pub const B3_MAINNET_CHAIN_ID: u64 = 8333;
pub const B3_TESTNET_CHAIN_ID: u64 = 1993;
pub const BSC_MAINNET_CHAIN_ID: u64 = 56;
pub const ARPA_CHAIN_ID: u64 = 4224;

pub fn supports_eip1559(chain_id: u64) -> bool {
    chain_id != LOOT_MAINNET_CHAIN_ID && chain_id != LOOT_TESTNET_CHAIN_ID
}

pub fn format_now_date() -> String {
    let fmt = "%Y-%m-%d %H:%M:%S";
    Local::now().format(fmt).to_string()
}

pub fn address_to_string(address: Address) -> String {
    to_checksum(&address, None)
}

pub fn random_address() -> Address {
    let nonce = rand::random::<u64>();
    Address::ZERO.create(nonce)
}

pub fn u256_to_vec(x: &U256) -> Vec<u8> {
    x.to_be_bytes_vec()
}

pub fn pad_to_bytes32(s: &[u8]) -> Option<[u8; 32]> {
    let s_len = s.len();

    if s_len > 32 {
        return None;
    }

    let mut result: [u8; 32] = Default::default();

    result[..s_len].clone_from_slice(s);

    Some(result)
}

pub fn pad_to_bytes32_fixed_bytes(s: &[u8]) -> Option<FixedBytes<32>> {
    let s_len = s.len();
    if s_len > 32 {
        return None;
    }
    let mut result: [u8; 32] = Default::default();
    result[..s_len].clone_from_slice(s);
    Some(FixedBytes::from(result))
}

pub fn bytes32_to_fixed_bytes(s: [u8; 32]) -> FixedBytes<32> {
    FixedBytes::from(s)
}

pub fn ser_bytes_in_hex_string<T, S>(v: &T, s: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[u8]>,
    S: serde::Serializer,
{
    // add "0x" prefix to the hex string
    s.serialize_str(&format!("0x{}", hex::encode(v.as_ref())))
}

pub fn ser_u256_in_dec_string<S>(v: &U256, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&format!("{}", v))
}

/// Converts an Ethereum address to the checksum encoding
/// Ref: <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-55.md>
pub fn to_checksum(addr: &Address, chain_id: Option<u8>) -> String {
    let prefixed_addr = match chain_id {
        Some(chain_id) => format!("{}0x{:x}", chain_id, addr),
        None => format!("{:x}", addr),
    };
    let hash = hex::encode(keccak256(prefixed_addr));
    let hash = hash.as_bytes();

    let addr_hex = hex::encode(addr);
    let addr_hex = addr_hex.as_bytes();

    addr_hex
        .iter()
        .zip(hash)
        .fold("0x".to_owned(), |mut encoded, (addr, hash)| {
            encoded.push(if *hash >= 56 {
                addr.to_ascii_uppercase() as char
            } else {
                addr.to_ascii_lowercase() as char
            });
            encoded
        })
}

// fn estimate(&self, base_fee: u128, rewards: &[Vec<u128>]) -> Eip1559Estimation;

/// The EIP-1559 fee estimator which is based on the work by [ethers-rs](https://github.com/gakonst/ethers-rs/blob/e0e79df7e9032e882fce4f47bcc25d87bceaec68/ethers-core/src/utils/mod.rs#L500) and [MyCrypto](https://github.com/MyCryptoHQ/MyCrypto/blob/master/src/services/ApiService/Gas/eip1559.ts)
pub fn eip1559_gas_price_estimator(base: u128, tips: &[Vec<u128>]) -> Eip1559Estimation {
    info!("base: {:?}", base);
    info!("tips: {:?}", tips);

    let max_priority_fee_per_gas = estimate_priority_fee(tips);

    fallback_eip1559_gas_price_estimator(base, max_priority_fee_per_gas)
}

pub fn fallback_eip1559_gas_price_estimator(
    base: u128,
    max_priority_fee_per_gas: u128,
) -> Eip1559Estimation {
    info!("base: {:?}", base);
    info!("max_priority_fee_per_gas: {:?}", max_priority_fee_per_gas);

    let potential_max_fee = base * 12 / 10;

    let max_fee_per_gas = if max_priority_fee_per_gas > potential_max_fee {
        max_priority_fee_per_gas + potential_max_fee
    } else {
        potential_max_fee
    };

    Eip1559Estimation {
        max_fee_per_gas,
        max_priority_fee_per_gas,
    }
}

fn estimate_priority_fee(rewards: &[Vec<u128>]) -> u128 {
    let mut rewards: Vec<u128> = rewards.iter().map(|r| r[0]).filter(|r| *r > 0).collect();
    if rewards.is_empty() {
        return 0;
    }
    if rewards.len() == 1 {
        return rewards[0];
    }
    // Sort the rewards as we will eventually take the median.
    rewards.sort();

    // A copy of the same vector is created for convenience to calculate percentage change
    // between subsequent fee values.
    let mut rewards_copy = rewards.clone();
    rewards_copy.rotate_left(1);

    let mut percentage_change: Vec<i128> = rewards
        .iter()
        .zip(rewards_copy.iter())
        .map(|(a, b)| {
            let a = *a as i128;
            let b = *b as i128;
            ((b - a) * 100) / a
        })
        .collect();
    percentage_change.pop();

    // Fetch the max of the percentage change, and that element's index.
    let max_change = percentage_change.iter().max().unwrap();
    let max_change_index = percentage_change
        .iter()
        .position(|&c| c == *max_change)
        .unwrap();

    // If we encountered a big change in fees at a certain position, then consider only
    // the values >= it.
    let values = if *max_change >= EIP1559_FEE_ESTIMATION_THRESHOLD_MAX_CHANGE.into()
        && (max_change_index >= (rewards.len() / 2))
    {
        rewards[max_change_index..].to_vec()
    } else {
        rewards
    };

    // Return the median.
    values[values.len() / 2]
}

#[cfg(test)]
pub mod util_tests {

    use alloy::primitives::Address;

    use crate::{address_to_string, format_now_date, random_address};

    #[test]
    fn test_format_now_date() {
        let res = format_now_date();
        println!("{:?}", res);
    }

    #[test]
    fn test_address_format() {
        let good_address_in_str = "0x0000000000000000000000000000000000000001";
        let address = good_address_in_str.parse::<Address>();
        assert!(address.is_ok());
        let address_as_str = address_to_string(address.unwrap());
        assert_eq!(address.unwrap(), address_as_str.parse::<Address>().unwrap());

        let bad_address_in_str = "0x1";
        let address = bad_address_in_str.parse::<Address>();
        assert!(address.is_err());
    }

    #[test]
    fn test_random_address() {
        for _ in 0..10 {
            let address = random_address();
            println!("{:?}", address);
        }
    }
}
