use chrono::Local;
use ethers_core::types::{Address, U256};

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
