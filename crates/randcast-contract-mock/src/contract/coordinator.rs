use super::errors::{CoordinatorError, CoordinatorResult};
use std::collections::HashMap;

pub struct Coordinator {
    /// Mapping of Ethereum Address => BLS public keys
    pub keys: HashMap<String, Vec<u8>>,
    /// Mapping of Ethereum Address => DKG Phase 1 Shares
    pub shares: HashMap<String, Vec<u8>>,
    /// Mapping of Ethereum Address => DKG Phase 2 Responses
    pub responses: HashMap<String, Vec<u8>>,
    /// Mapping of Ethereum Address => DKG Phase 3 Justifications
    pub justifications: HashMap<String, Vec<u8>>,
    /// List of registered Ethereum keys (used for conveniently fetching data)
    pub participants: Vec<String>,
    // The duration of each phase
    pub phase_duration: usize,
    /// The epoch of the group
    pub epoch: usize,
    /// The threshold of the DKG
    pub threshold: usize,
    /// If it's 0 then the DKG is still pending start. If >0, it is the DKG's start block
    pub start_block: usize,
    /// for mock
    pub block_height: usize,
}

impl Coordinator {
    pub fn new(epoch: usize, threshold: usize, phase_duration: usize) -> Self {
        Coordinator {
            keys: HashMap::new(),
            shares: HashMap::new(),
            responses: HashMap::new(),
            justifications: HashMap::new(),
            participants: vec![],
            start_block: 0,
            phase_duration,
            epoch,
            threshold,
            block_height: 0,
        }
    }
}

pub trait Transactions {
    /// Kickoff function which starts the counter
    fn initialize(
        &mut self,
        block_height: usize,
        members: Vec<(String, usize, Vec<u8>)>,
    ) -> CoordinatorResult<()>;

    /// Participant publishes their data and depending on the phase the data gets inserted
    /// in the shares, responses or justifications mapping. Reverts if the participant
    /// has already published their data for a phase or if the DKG has ended.
    fn publish(&mut self, id_address: String, value: Vec<u8>) -> CoordinatorResult<()>;
}

pub trait Views {
    // Helpers to fetch data in the mappings. If a participant has registered but not
    // published their data for a phase, the array element at their index is expected to be 0

    /// Gets the participants' shares
    fn get_shares(&self) -> CoordinatorResult<Vec<Vec<u8>>>;

    /// Gets the participants' responses
    fn get_responses(&self) -> CoordinatorResult<Vec<Vec<u8>>>;

    /// Gets the participants' justifications
    fn get_justifications(&self) -> CoordinatorResult<Vec<Vec<u8>>>;

    /// Gets the participants' ethereum addresses
    fn get_participants(&self) -> CoordinatorResult<Vec<String>>;

    /// Gets the participants' BLS keys along with the thershold of the DKG
    fn get_bls_keys(&self) -> CoordinatorResult<(usize, Vec<Vec<u8>>)>;

    /// Returns the current phase of the DKG.
    fn in_phase(&self) -> CoordinatorResult<usize>;
}

pub trait Internal {
    /// A registered participant is one whose pubkey's length > 0
    fn only_allowed(&self, id_address: &str) -> CoordinatorResult<()>;

    /// The DKG starts when startBlock > 0
    fn only_when_not_started(&self) -> CoordinatorResult<()>;
}

pub trait MockHelper {
    fn mine(&mut self, block_number: usize);
}

impl MockHelper for Coordinator {
    fn mine(&mut self, block_number: usize) {
        self.block_height += block_number;

        println!("coordinator block_height: {}", self.block_height);
    }
}

impl Internal for Coordinator {
    fn only_allowed(&self, id_address: &str) -> CoordinatorResult<()> {
        if !self.participants.iter().any(|x| x == id_address) {
            return Err(CoordinatorError::NotAllowlisted);
        }

        Ok(())
    }

    fn only_when_not_started(&self) -> CoordinatorResult<()> {
        if self.start_block != 0 {
            return Err(CoordinatorError::AlreadyStarted);
        }

        Ok(())
    }
}

impl Transactions for Coordinator {
    fn initialize(
        &mut self,
        block_height: usize,
        members: Vec<(String, usize, Vec<u8>)>,
    ) -> CoordinatorResult<()> {
        self.only_when_not_started()?;

        self.start_block = block_height;

        self.block_height = block_height;

        members.into_iter().for_each(|(address, _, key)| {
            self.participants.push(address.clone());

            self.keys.insert(address, key);
        });

        println!("coordinator deployed. block_height: {}", self.block_height);

        Ok(())
    }

    fn publish(&mut self, id_address: String, value: Vec<u8>) -> CoordinatorResult<()> {
        self.only_allowed(&id_address)?;

        let blocks_since_start = self.block_height - self.start_block;

        if blocks_since_start <= self.phase_duration {
            if self.shares.contains_key(&id_address) {
                return Err(CoordinatorError::SharesExisted);
            }

            self.shares.insert(id_address, value);
        } else if blocks_since_start <= 2 * self.phase_duration {
            if self.responses.contains_key(&id_address) {
                return Err(CoordinatorError::ResponsesExisted);
            }

            self.responses.insert(id_address, value);
        } else if blocks_since_start <= 3 * self.phase_duration {
            if self.justifications.contains_key(&id_address) {
                return Err(CoordinatorError::JustificationsExisted);
            }

            self.justifications.insert(id_address, value);
        } else {
            return Err(CoordinatorError::DKGPublishEnded);
        }

        Ok(())
    }
}

impl Views for Coordinator {
    fn get_shares(&self) -> CoordinatorResult<Vec<Vec<u8>>> {
        Ok(self.shares.values().cloned().collect::<Vec<_>>())
    }

    fn get_responses(&self) -> CoordinatorResult<Vec<Vec<u8>>> {
        Ok(self.responses.values().cloned().collect::<Vec<_>>())
    }

    fn get_justifications(&self) -> CoordinatorResult<Vec<Vec<u8>>> {
        Ok(self.justifications.values().cloned().collect::<Vec<_>>())
    }

    fn get_participants(&self) -> CoordinatorResult<Vec<String>> {
        Ok(self.participants.clone())
    }

    fn get_bls_keys(&self) -> CoordinatorResult<(usize, Vec<Vec<u8>>)> {
        let keys = self
            .participants
            .iter()
            .map(|p| self.keys.get(p).unwrap().clone())
            .collect::<Vec<_>>();

        Ok((self.threshold, keys))
    }

    fn in_phase(&self) -> CoordinatorResult<usize> {
        let blocks_since_start = self.block_height - self.start_block;

        if blocks_since_start <= self.phase_duration {
            return Ok(0);
        }

        if blocks_since_start <= 2 * self.phase_duration {
            return Ok(1);
        }

        if blocks_since_start <= 3 * self.phase_duration {
            return Ok(2);
        }

        if blocks_since_start <= 4 * self.phase_duration {
            return Ok(3);
        }

        Err(CoordinatorError::DKGEnded)
    }
}
