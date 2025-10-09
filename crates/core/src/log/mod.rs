use crate::ser_bytes_in_hex_string;
use crate::Group;
use crate::TaskType;
use alloy::primitives::{Address, B256};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt::Display;
use threshold_bls::group::Curve;
use threshold_bls::serialize::point_to_hex;

pub mod encoder;

#[derive(Serialize, Deserialize, Debug)]
pub enum LogType {
    DKGKeyGenerated,
    NodeRegistered,
    NodeRegisterFailed,
    NodeActivated,
    NodeActivationFailed,
    TaskReceived,
    DKGGroupingStarted,
    DKGGroupingFinished,
    DKGGroupingAborted,
    DKGGroupingFailed,
    DKGGroupingCommitted,
    DKGGroupingCommitFailed,
    DKGGroupingAvailable,
    DKGGroupingTwisted,
    DKGGroupingMemberMisMatch,
    DKGPostProcessFinished,
    DKGPostProcessGroupRelayFinished,
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
    ProviderReconnected,
    ListenerRestarted,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Payload<'a> {
    pub log_type: LogType,
    pub message: &'a str,
    pub chain_id: Option<u64>,
    pub group_log: Option<GroupLog>,
    pub task_log: Option<TaskLog<'a>>,
    pub transaction_receipt_log: Option<TransactionReceiptLog>,
}

impl Display for Payload<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        json!(self).fmt(f)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GroupLog {
    pub index: usize,
    pub epoch: usize,
    pub size: usize,
    pub threshold: usize,
    pub state: bool,
    pub public_key: Option<String>,
    pub members: Vec<MemberLog>,
    pub committers: Vec<Address>,
    pub relayed_chain_id: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MemberLog {
    pub index: usize,
    pub id_address: Address,
    pub rpc_endpoint: Option<String>,
    pub partial_public_key: Option<String>,
}

impl<C: Curve> From<&Group<C>> for GroupLog {
    fn from(group: &Group<C>) -> Self {
        GroupLog {
            index: group.index,
            epoch: group.epoch,
            size: group.size,
            threshold: group.threshold,
            state: group.state,
            public_key: group.public_key.as_ref().map(point_to_hex),
            members: group
                .members
                .iter()
                .map(|(id_address, member)| MemberLog {
                    index: member.index,
                    id_address: *id_address,
                    rpc_endpoint: member.rpc_endpoint.clone(),
                    partial_public_key: member.partial_public_key.as_ref().map(point_to_hex),
                })
                .collect(),
            committers: group.committers.clone(),
            relayed_chain_id: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskLog<'a> {
    #[serde(serialize_with = "ser_bytes_in_hex_string")]
    pub request_id: &'a [u8],
    pub task_type: TaskType,
    pub task_json: Value,
    pub committer_id_address: Option<Address>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionReceiptLog {
    pub transaction_hash: B256,
    pub gas_used: u64,
    pub effective_gas_price: u128,
}

pub fn build_general_payload<'a>(
    log_type: LogType,
    message: &'a str,
    chain_id: Option<u64>,
) -> Payload<'a> {
    Payload {
        log_type,
        message,
        chain_id,
        group_log: None,
        task_log: None,
        transaction_receipt_log: None,
    }
}

pub fn build_group_related_payload<'a, C: Curve>(
    log_type: LogType,
    message: &'a str,
    chain_id: u64,
    group: &'a Group<C>,
) -> Payload<'a> {
    Payload {
        log_type,
        message,
        chain_id: Some(chain_id),
        group_log: Some(group.into()),
        task_log: None,
        transaction_receipt_log: None,
    }
}

pub fn build_task_related_payload<'a>(
    log_type: LogType,
    message: &'a str,
    chain_id: u64,
    request_id: &'a [u8],
    task_type: TaskType,
    task_json: Value,
    committer_id_address: Option<Address>,
) -> Payload<'a> {
    Payload {
        log_type,
        message,
        chain_id: Some(chain_id),
        group_log: None,
        task_log: Some(TaskLog {
            request_id,
            task_type,
            task_json,
            committer_id_address,
        }),
        transaction_receipt_log: None,
    }
}

pub fn build_transaction_receipt_payload<'a>(
    log_type: LogType,
    message: &'a str,
    chain_id: u64,
    transaction_hash: B256,
    gas_used: u64,
    effective_gas_price: u128,
) -> Payload<'a> {
    Payload {
        log_type,
        message,
        chain_id: Some(chain_id),
        group_log: None,
        task_log: None,
        transaction_receipt_log: Some(TransactionReceiptLog {
            transaction_hash,
            gas_used,
            effective_gas_price,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_group_related_transaction_receipt_payload<'a, C: Curve>(
    log_type: LogType,
    message: &'a str,
    chain_id: u64,
    group: &'a Group<C>,
    relayed_chain_id: Option<u64>,
    transaction_hash: B256,
    gas_used: u64,
    effective_gas_price: u128,
) -> Payload<'a> {
    let mut group_log: GroupLog = group.into();
    group_log.relayed_chain_id = relayed_chain_id;

    Payload {
        log_type,
        message,
        chain_id: Some(chain_id),
        group_log: Some(group_log),
        task_log: None,
        transaction_receipt_log: Some(TransactionReceiptLog {
            transaction_hash,
            gas_used,
            effective_gas_price,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_task_related_transaction_receipt_payload<'a>(
    log_type: LogType,
    message: &'a str,
    chain_id: u64,
    request_id: &'a [u8],
    task_type: TaskType,
    task_json: Value,
    transaction_hash: B256,
    gas_used: u64,
    effective_gas_price: u128,
) -> Payload<'a> {
    Payload {
        log_type,
        message,
        chain_id: Some(chain_id),
        group_log: None,
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
    use crate::BLSTaskType;

    use super::*;

    #[test]
    fn test_build_json_payload() {
        let payload = build_general_payload(LogType::TaskReceived, "Request received", Some(1));
        println!("{}", payload);

        let request_id = vec![0, 1, 2, 3, 4, 5, 6, 7];

        let payload = build_task_related_payload(
            LogType::TaskReceived,
            "Request received",
            1,
            &request_id,
            TaskType::BLS(BLSTaskType::Randomness),
            json!({ "task": "Randomness" }),
            None,
        );
        println!("{}", payload);

        let payload = build_task_related_transaction_receipt_payload(
            LogType::FulfillmentFinished,
            "Fulfillment finished",
            1,
            &request_id,
            TaskType::BLS(BLSTaskType::Randomness),
            json!({ "task": "Randomness" }),
            B256::ZERO,
            0,
            0,
        );
        println!("{}", payload);
    }
}
