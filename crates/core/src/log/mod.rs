use crate::ser_bytes_in_hex_string;
use crate::ser_u256_in_dec_string;
use crate::BLSTaskType;
use ethers_core::types::{Address, H256, U256};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt::Display;

pub mod encoder;

#[derive(Serialize, Deserialize, Debug)]
pub enum LogType {
    RequestReceived,
    PartialSignatureFinished,
    PartialSignatureFailed,
    PartialSignatureSent,
    PartialSignatureSendingRejected,
    PartialSignatureSendingFailed,
    AggregatedSignatureFinished,
    AggregatedSignatureFailed,
    FulfillmentFinished,
    FulfillmentFailed,
    ListenerInterrupted,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Payload<'a> {
    pub log_type: LogType,
    pub message: &'a str,
    pub chain_id: Option<usize>,
    pub task_log: Option<TaskLog<'a>>,
    pub transaction_receipt_log: Option<TransactionReceiptLog>,
}

impl Display for Payload<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        json!(self).fmt(f)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskLog<'a> {
    #[serde(serialize_with = "ser_bytes_in_hex_string")]
    pub request_id: &'a [u8],
    pub task_type: BLSTaskType,
    pub task_json: Value,
    pub committer_id_address: Option<Address>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionReceiptLog {
    pub transaction_hash: H256,
    #[serde(serialize_with = "ser_u256_in_dec_string")]
    pub gas_used: U256,
    #[serde(serialize_with = "ser_u256_in_dec_string")]
    pub effective_gas_price: U256,
}

pub fn build_general_payload(log_type: LogType, message: &str, chain_id: Option<usize>) -> Payload {
    Payload {
        log_type,
        message,
        chain_id,
        task_log: None,
        transaction_receipt_log: None,
    }
}

pub fn build_request_related_payload<'a>(
    log_type: LogType,
    message: &'a str,
    chain_id: usize,
    request_id: &'a [u8],
    task_type: BLSTaskType,
    task_json: Value,
    committer_id_address: Option<Address>,
) -> Payload<'a> {
    Payload {
        log_type,
        message,
        chain_id: Some(chain_id),
        task_log: Some(TaskLog {
            request_id,
            task_type,
            task_json,
            committer_id_address,
        }),
        transaction_receipt_log: None,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_transaction_receipt_log_payload<'a>(
    log_type: LogType,
    message: &'a str,
    chain_id: usize,
    request_id: &'a [u8],
    task_type: BLSTaskType,
    task_json: Value,
    transaction_hash: H256,
    gas_used: U256,
    effective_gas_price: U256,
) -> Payload<'a> {
    Payload {
        log_type,
        message,
        chain_id: Some(chain_id),
        task_log: Some(TaskLog {
            request_id,
            task_type,
            task_json,
            committer_id_address: None,
        }),
        transaction_receipt_log: Some(TransactionReceiptLog {
            transaction_hash,
            gas_used,
            effective_gas_price,
        }),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_build_json_payload() {
        let payload = build_general_payload(LogType::RequestReceived, "Request received", Some(1));
        println!("{}", payload);

        let request_id = vec![0, 1, 2, 3, 4, 5, 6, 7];

        let payload = build_request_related_payload(
            LogType::RequestReceived,
            "Request received",
            1,
            &request_id,
            BLSTaskType::Randomness,
            json!({ "task": "Randomness" }),
            None,
        );
        println!("{}", payload);

        let payload = build_transaction_receipt_log_payload(
            LogType::FulfillmentFinished,
            "Fulfillment finished",
            1,
            &request_id,
            BLSTaskType::Randomness,
            json!({ "task": "Randomness" }),
            H256::zero(),
            U256::zero(),
            U256::zero(),
        );
        println!("{}", payload);
    }
}
