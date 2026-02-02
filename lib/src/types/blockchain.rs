use super::{Block, Transaction, TransactionOutput};
use crate::U256;
use crate::error::{BtcError, Result};
use crate::sha256::Hash;
use crate::util::MerkleRoot;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Blockchain {
    utxos: HashMap<Hash, (bool, TransactionOutput)>,
    blocks: Vec<Block>,
    target: U256,
    #[serde(default, skip_serializing)]
    mempool: Vec<(DateTime<Utc>, Transaction)>,
}

impl Blockchain {
    pub fn new() -> Self {
        Blockchain {
            utxos: HashMap::new(),
            blocks: vec![],
            target: crate::MIN_TARGET,
            mempool: vec![],
        }
    }

    pub fn utxos(&self) -> &HashMap<Hash, (bool, TransactionOutput)> {
        &self.utxos
    }

    pub fn target(&self) -> U256 {
        self.target
    }

    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.iter()
    }

    pub fn block_height(&self) -> u64 {
        self.blocks.len() as u64
    }

    pub fn mempool(&self) -> &[(DateTime<Utc>, Transaction)] {
        &self.mempool
    }

    pub fn rebuild_utxos(&mut self) {
        for block in &self.blocks {
            for transaction in &block.transactions {
                for input in &transaction.inputs {
                    self.utxos.remove(&input.prev_transaction_output_hash);
                }
                for output in transaction.outputs.iter() {
                    self.utxos
                        .insert(transaction.hash(), (false, output.clone()));
                }
            }
        }
    }

    pub fn try_adjust_target(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        if self.blocks.len() % crate::DIFFICULTY_UPDATE_INTERVAL as usize != 0 {
            return;
        }
        // measure the time it took to mine the last crate::DIFFICULTY_UPDATE_INTERVAL blocks
        let start_time = self.blocks
            [self.blocks.len() - crate::DIFFICULTY_UPDATE_INTERVAL as usize]
            .header
            .timestamp;
        let end_time = self.blocks.last().unwrap().header.timestamp;
        let time_diff = end_time - start_time;
        let time_diff_seconds = time_diff.num_seconds();
        let target_seconds = crate::IDEAL_BLOCK_TIME * crate::DIFFICULTY_UPDATE_INTERVAL;
        let new_target = BigDecimal::parse_bytes(&self.target.to_string().as_bytes(), 10)
            .expect("BUG: impossible")
            * (BigDecimal::from(time_diff_seconds) / BigDecimal::from(target_seconds));
        let new_target_str = new_target
            .to_string()
            .split('.')
            .next()
            .expect("BUG: expected a decimal point")
            .to_owned();
        let new_target: U256 = U256::from_str_radix(&new_target_str, 10).expect("BUG: impossible");
        let new_target = if new_target < self.target / 4 {
            self.target / 4
        } else if new_target > self.target * 4 {
            self.target * 4
        } else {
            new_target
        };
        self.target = new_target.min(crate::MIN_TARGET);

        if self.blocks.len() % crate::DIFFICULTY_UPDATE_INTERVAL as usize != 0 {
            return;
        }
    }

    pub fn add_block(&mut self, block: Block) -> Result<()> {
        if self.blocks.is_empty() && block.header.prev_block_hash != Hash::zero() {
            println!("zero hash");
            return Err(BtcError::InvalidBlock);
        } else {
            let last_block = self.blocks.last().unwrap();
            if block.header.prev_block_hash != last_block.hash() {
                println!("prev hash is wrong");
                return Err(BtcError::InvalidBlock);
            }

            if !block.header.hash().matches_target(block.header.target) {
                println!("does not match target");
                return Err(BtcError::InvalidBlock);
            }

            let calculated_merkle_root = MerkleRoot::calculate(&block.transactions);
            if calculated_merkle_root != block.header.merkle_root {
                println!("invalid merkle root");
                return Err(BtcError::InvalidMerkleRoot);
            }

            if block.header.timestamp <= last_block.header.timestamp {
                return Err(BtcError::InvalidBlock);
            }

            block.verify_transactions(self.block_height(), &self.utxos)?;
        }
        let block_transactions: HashSet<_> =
            block.transactions.iter().map(|tx| tx.hash()).collect();
        self.mempool
            .retain(|(_, tx)| !block_transactions.contains(&tx.hash()));
        self.blocks.push(block);
        self.try_adjust_target();
        Ok(())
    }

    pub fn add_to_mempool(&mut self, transaction: Transaction) -> Result<()> {
        let mut known_inputs = HashSet::new();
        for input in &transaction.inputs {
            if !self.utxos.contains_key(&input.prev_transaction_output_hash) {
                return Err(BtcError::InvalidTransaction);
            }
            if known_inputs.contains(&input.prev_transaction_output_hash) {
                return Err(BtcError::InvalidTransaction);
            }
            known_inputs.insert(input.prev_transaction_output_hash);
        }

        // check if any of the utxos have the bool mark set to true
        // and if so, find the transaction that references them in mempool,
        // remove it, and set all the utxos it references to false
        for input in &transaction.inputs {
            if let Some((true, _)) = self.utxos.get(&input.prev_transaction_output_hash) {
                let referencing_transaction =
                    self.mempool
                        .iter()
                        .enumerate()
                        .find(|(_, (_, transaction))| {
                            transaction
                                .outputs
                                .iter()
                                .any(|output| output.hash() == input.prev_transaction_output_hash)
                        });
                if let Some((idx, (_, referencing_transaction_local))) = referencing_transaction {
                    for input in &referencing_transaction_local.inputs {
                        self.utxos
                            .entry(input.prev_transaction_output_hash)
                            .and_modify(|(marked, _)| {
                                *marked = false;
                            });
                    }
                    self.mempool.remove(idx);
                } else {
                    self.utxos
                        .entry(input.prev_transaction_output_hash)
                        .and_modify(|(marked, _)| *marked = false);
                }
            }
        }
        let all_inputs = transaction
            .inputs
            .iter()
            .map(|input| {
                self.utxos
                    .get(&input.prev_transaction_output_hash)
                    .expect("BUG: impossible")
                    .1
                    .value
            })
            .sum::<u64>();
        let all_outputs: u64 = transaction.outputs.iter().map(|output| output.value).sum();
        if all_inputs < all_outputs {
            return Err(BtcError::InvalidTransaction);
        }
        for input in &transaction.inputs {
            self.utxos
                .entry(input.prev_transaction_output_hash)
                .and_modify(|(marked, _)| {
                    *marked = true;
                });
        }

        self.mempool.push((Utc::now(), transaction));
        self.mempool.sort_by_key(|(_, transaction)| {
            let all_inputs = transaction
                .inputs
                .iter()
                .map(|input| {
                    self.utxos
                        .get(&input.prev_transaction_output_hash)
                        .expect("BUG: impossible")
                        .1
                        .value
                })
                .sum::<u64>();
            let all_outputs: u64 = transaction.outputs.iter().map(|output| output.value).sum();
            let miner_fee = all_inputs - all_outputs;
            miner_fee
        });
        Ok(())
    }

    pub fn cleanup_mempool(&mut self) {
        let now = Utc::now();
        let mut utxo_hashes_to_unmark: Vec<Hash> = vec![];
        self.mempool.retain(|(timestamp, transaction)| {
            if now - *timestamp
                > chrono::Duration::seconds(crate::MAX_MEMPOOL_TRANSACTION_AGE as i64)
            {
                utxo_hashes_to_unmark.extend(
                    transaction
                        .inputs
                        .iter()
                        .map(|input| input.prev_transaction_output_hash),
                );
                false
            } else {
                true
            }
        });
        for hash in utxo_hashes_to_unmark {
            self.utxos.entry(hash).and_modify(|(marked, _)| {
                *marked = false;
            });
        }
    }
}
