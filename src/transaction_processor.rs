use crate::transaction::Transaction;
use crate::block::Block;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub struct TransactionProcessor {
    pending_transactions: Arc<Mutex<Vec<Transaction>>>,
    processed_transactions: Arc<Mutex<HashMap<String, bool>>>,
}

impl TransactionProcessor {
    pub fn new() -> Self {
        TransactionProcessor {
            pending_transactions: Arc::new(Mutex::new(Vec::new())),
            processed_transactions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Add a new transaction to the pending pool
    pub fn add_transaction(&self, transaction: Transaction) {
        let mut pending = self.pending_transactions.lock().unwrap();
        pending.push(transaction);
    }

    // Process transactions in parallel
    pub fn process_pending_transactions(&self) -> Vec<Transaction> {
        let pending = self.pending_transactions.lock().unwrap();
        let processed = self.processed_transactions.clone();

        // Convert pending transactions to a parallel iterator
        let validated_transactions: Vec<Transaction> = pending.par_iter()
            .filter_map(|tx| {
                // Check if transaction was already processed
                let processed_map = processed.lock().unwrap();
                if processed_map.contains_key(&tx.id) {
                    return None;
                }

                // Validate transaction
                if self.validate_transaction(tx) {
                    Some(tx.clone())
                } else {
                    None
                }
            })
            .collect();

        // Update processed transactions map
        let mut processed_map = processed.lock().unwrap();
        for tx in &validated_transactions {
            processed_map.insert(tx.id.clone(), true);
        }

        validated_transactions
    }

    // Validate a single transaction
    fn validate_transaction(&self, transaction: &Transaction) -> bool {
        // Add your transaction validation logic here
        // For example:
        // - Check if sender has sufficient balance
        // - Verify transaction signature
        // - Check for double spending
        // - Validate transaction format
        true // Placeholder return
    }

    // Group transactions for parallel processing
    pub fn group_transactions(&self, transactions: Vec<Transaction>) -> Vec<Vec<Transaction>> {
        let mut groups: Vec<Vec<Transaction>> = Vec::new();
        let mut current_group: Vec<Transaction> = Vec::new();
        let max_group_size = 1024; // Similar to Solana's transaction processing

        for tx in transactions {
            if current_group.len() >= max_group_size {
                groups.push(current_group);
                current_group = Vec::new();
            }
            current_group.push(tx);
        }

        if !current_group.is_empty() {
            groups.push(current_group);
        }

        groups
    }

    // Calculate transaction fees
    pub fn calculate_fee(&self, transaction: &Transaction) -> u64 {
        // Implement dynamic fee calculation based on:
        // - Network congestion
        // - Transaction size
        // - Priority
        // For now, return a minimal fee
        100 // Base fee in lamports (like Solana)
    }
} 