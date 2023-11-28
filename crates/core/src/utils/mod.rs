use chrono::Local;
use ethers_core::types::{Address, I256, U256};
use log::info;

/// The threshold max change/difference (in %) at which we will ignore the fee history values
/// under it.
pub const EIP1559_FEE_ESTIMATION_THRESHOLD_MAX_CHANGE: i64 = 200;

pub fn format_now_date() -> String {
    let fmt = "%Y-%m-%d %H:%M:%S";
    Local::now().format(fmt).to_string()
}

pub fn address_to_string(address: Address) -> String {
    format!("{:?}", address)
}

pub fn u256_to_vec(x: &U256) -> Vec<u8> {
    let mut x_bytes = vec![0u8; 32];
    x.to_big_endian(&mut x_bytes);
    x_bytes
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

/// The EIP-1559 fee estimator which is based on the work by [ethers-rs](https://github.com/gakonst/ethers-rs/blob/e0e79df7e9032e882fce4f47bcc25d87bceaec68/ethers-core/src/utils/mod.rs#L500) and [MyCrypto](https://github.com/gakonst/ethers-rs/blob/e0e79df7e9032e882fce4f47bcc25d87bceaec68/ethers-core/src/utils/mod.rs#L500)
pub fn eip1559_gas_price_estimator(base: U256, tips: Vec<Vec<U256>>) -> (U256, U256) {
    info!("base: {:?}", base);
    info!("tips: {:?}", tips);

    let max_priority_fee_per_gas = estimate_priority_fee(tips);

    let potential_max_fee = base * 12 / 10;

    let max_fee_per_gas = if max_priority_fee_per_gas > potential_max_fee {
        max_priority_fee_per_gas + potential_max_fee
    } else {
        potential_max_fee
    };
    (max_fee_per_gas, max_priority_fee_per_gas)
}

fn estimate_priority_fee(rewards: Vec<Vec<U256>>) -> U256 {
    let mut rewards: Vec<U256> = rewards
        .iter()
        .map(|r| r[0])
        .filter(|r| *r > U256::zero())
        .collect();
    if rewards.is_empty() {
        return U256::zero();
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

    let mut percentage_change: Vec<I256> = rewards
        .iter()
        .zip(rewards_copy.iter())
        .map(|(a, b)| {
            let a = I256::try_from(*a).expect("priority fee overflow");
            let b = I256::try_from(*b).expect("priority fee overflow");
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

    use ethers_core::types::Address;

    use crate::{address_to_string, format_now_date};

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
}
