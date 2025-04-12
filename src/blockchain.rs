//  blockchain.rs

// Import necessary types
use crate::TXOutput;                      // Transaction outputs
use crate::{Block, Transaction};          // Block and Transaction types
use data_encoding::HEXLOWER;              // For hex encoding of hashes
use sled::transaction::TransactionResult; // Database transaction type
use sled::{Db, Tree};                     // Database types
use std::collections::HashMap;            // For tracking UTXOs
use std::env::current_dir;                // For locating the database
use std::sync::{Arc, RwLock};             // For thread-safe access to shared state
use crate::proof_of_history::ProofOfHistory;
use crate::transaction_processor::TransactionProcessor;

// Key used in the database to store the hash of the latest block (tip of the chain)
const TIP_BLOCK_HASH_KEY: &str = "tip_block_hash";
// Name of the database tree/bucket that stores blocks
const BLOCKS_TREE: &str = "blocks";

// The main Blockchain struct - represents the entire blockchain
// Implements Clone to allow easy copying of the blockchain reference
#[derive(Clone)]
pub struct Blockchain {
    tip_hash: Arc<RwLock<String>>, // Hash of the latest block, thread-safe with Arc<RwLock>
    db: Db,                        // Database connection for persistent storage
    poh: ProofOfHistory,
    transaction_processor: TransactionProcessor,
}

impl Blockchain {
    
    // Create a new blockchain or load an existing one
    // If no blockchain exists, creates a genesis block with a reward to the given address
    pub fn create_blockchain(genesis_address: &str) -> Blockchain {
        // Open the database
        let db = sled::open(current_dir().unwrap().join("data")).unwrap();
        // Open the blocks tree/bucket
        let blocks_tree = db.open_tree(BLOCKS_TREE).unwrap();

        // Try to get the tip hash from the database
        let data = blocks_tree.get(TIP_BLOCK_HASH_KEY).unwrap();
        let tip_hash;
        
        if data.is_none() {
            // No blockchain exists - create a genesis block
            // Create a coinbase transaction (mining reward) to the genesis address
            let coinbase_tx = Transaction::new_coinbase_tx(genesis_address);
            // Generate the genesis block with this transaction
            let block = Block::generate_genesis_block(&coinbase_tx);
            // Store the block in the database
            Self::update_blocks_tree(&blocks_tree, &block);
            // Set the tip hash to the genesis block's hash
            tip_hash = String::from(block.get_hash());
        } else {
            // Blockchain exists - load the tip hash
            tip_hash = String::from_utf8(data.unwrap().to_vec()).unwrap();
        }
        
        // Create and return the Blockchain instance
        Blockchain {
            tip_hash: Arc::new(RwLock::new(tip_hash)), // Wrap in Arc<RwLock> for thread safety
            db,                                        // Store database connection
            poh: ProofOfHistory::new(),
            transaction_processor: TransactionProcessor::new(),
        }
    }

    // Helper method to update the blocks tree in the database
    // Stores a block and updates the tip hash atomically
    fn update_blocks_tree(blocks_tree: &Tree, block: &Block) {
        let block_hash = block.get_hash();
        // Use a database transaction to ensure atomicity
        let _: TransactionResult<(), ()> = blocks_tree.transaction(|tx_db| {
            let _ = tx_db.insert(block_hash, block.clone()); // Store the block
            let _ = tx_db.insert(TIP_BLOCK_HASH_KEY, block_hash); // Update the tip hash
            Ok(()) // Commit the transaction
        });
    }

    // Open an existing blockchain
    // Fails if no blockchain exists
    pub fn new_blockchain() -> Blockchain {
        // Open the database
        let db = sled::open(current_dir().unwrap().join("data")).unwrap();
        // Open the blocks tree
        let blocks_tree = db.open_tree(BLOCKS_TREE).unwrap();
        // Get the tip hash - expect to fail if no blockchain exists
        let tip_bytes = blocks_tree
            .get(TIP_BLOCK_HASH_KEY)
            .unwrap()
            .expect("No existing blockchain found. Create one first.");
        // Convert the tip hash from bytes to string
        let tip_hash = String::from_utf8(tip_bytes.to_vec()).unwrap();
        
        // Create and return the Blockchain instance
        Blockchain {
            tip_hash: Arc::new(RwLock::new(tip_hash)),
            db,
            poh: ProofOfHistory::new(),
            transaction_processor: TransactionProcessor::new(),
        }
    }

    // Get the database connection
    pub fn get_db(&self) -> &Db {
        &self.db
    }

    // Get the hash of the latest block (tip)
    pub fn get_tip_hash(&self) -> String {
        // Acquire a read lock and clone the hash
        self.tip_hash.read().unwrap().clone()
    }

    // Set a new tip hash
    // Used when adding blocks to the chain
    pub fn set_tip_hash(&self, new_tip_hash: &str) {
        // Acquire a write lock and update the hash
        let mut tip_hash = self.tip_hash.write().unwrap();
        *tip_hash = String::from(new_tip_hash)
    }

    // Mine a new block with the given transactions
    // This is called when a miner wants to add a new block to the chain
    pub fn mine_block(&self, transactions: &[Transaction]) -> Block {
        // Verify all transactions are valid
        for trasaction in transactions {
            if trasaction.verify(self) == false {
                panic!("ERROR: Invalid transaction")
            }
        }
        // Get the current height of the blockchain
        let best_height = self.get_best_height();

        // Create a new block with the transactions
        // The block links to the current tip and has height = best_height + 1
        let block = Block::new_block(self.get_tip_hash(), transactions, best_height + 1);
        let block_hash = block.get_hash();

        // Store the block in the database and update the tip
        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        Self::update_blocks_tree(&blocks_tree, &block);
        self.set_tip_hash(block_hash);
        
        // Return the new block
        block
    }

    // Create an iterator for traversing the blockchain from newest to oldest block
    pub fn iterator(&self) -> BlockchainIterator {
        BlockchainIterator::new(self.get_tip_hash(), self.db.clone())
    }

    // Find all unspent transaction outputs (UTXOs) in the blockchain
    // Returns a map from transaction ID to the list of unspent outputs
    // This is a key method for calculating wallet balances and available funds
    pub fn find_utxo(&self) -> HashMap<String, Vec<TXOutput>> {
        // Map to store unspent outputs
        let mut utxo: HashMap<String, Vec<TXOutput>> = HashMap::new();
        // Map to track which outputs have been spent
        let mut spent_txos: HashMap<String, Vec<usize>> = HashMap::new();

        // Create an iterator to traverse the blockchain
        let mut iterator = self.iterator();
        
        // Iterate through all blocks from newest to oldest
        loop {
            let option = iterator.next();
            if option.is_none() {
                break; // Reached the end of the blockchain
            }
            
            let block = option.unwrap();
            
            // For each transaction in the block
            'outer: for tx in block.get_transactions() {
                let txid_hex = HEXLOWER.encode(tx.get_id()); // Get transaction ID as hex
                
                // Process each output in the transaction
                for (idx, out) in tx.get_vout().iter().enumerate() {
                    // Check if this output has been spent
                    if let Some(outs) = spent_txos.get(txid_hex.as_str()) {
                        for spend_out_idx in outs {
                            if idx.eq(spend_out_idx) {
                                // This output has been spent - skip the rest of outputs
                                continue 'outer;
                            }
                        }
                    }
                    
                    // This output is unspent - add it to the UTXO map
                    if utxo.contains_key(txid_hex.as_str()) {
                        // Add to existing entry
                        utxo.get_mut(txid_hex.as_str()).unwrap().push(out.clone());
                    } else {
                        // Create new entry
                        utxo.insert(txid_hex.clone(), vec![out.clone()]);
                    }
                }
                
                // Skip inputs for coinbase transactions (they have no real inputs)
                if tx.is_coinbase() {
                    continue;
                }
                
                // Mark all outputs referenced by this transaction's inputs as spent
                for txin in tx.get_vin() {
                    let txid_hex = HEXLOWER.encode(txin.get_txid());
                    
                    // Add this output index to the spent outputs map
                    if spent_txos.contains_key(txid_hex.as_str()) {
                        spent_txos
                            .get_mut(txid_hex.as_str())
                            .unwrap()
                            .push(txin.get_vout());
                    } else {
                        spent_txos.insert(txid_hex, vec![txin.get_vout()]);
                    }
                }
            }
        }
        
        // Return the map of unspent outputs
        utxo
    }

   
    pub fn find_transaction(&self, txid: &[u8]) -> Option<Transaction> {
        let mut iterator = self.iterator();
        loop {
            let option = iterator.next();
            if option.is_none() {
                break;
            }
            let block = option.unwrap();
            for transaction in block.get_transactions() {
                if txid.eq(transaction.get_id()) {
                    return Some(transaction.clone());
                }
            }
        }
        None
    }

  
    pub fn add_block(&self, block: &Block) {
        let block_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        if let Some(_) = block_tree.get(block.get_hash()).unwrap() {
            return;
        }
        let _: TransactionResult<(), ()> = block_tree.transaction(|tx_db| {
            let _ = tx_db.insert(block.get_hash(), block.serialize()).unwrap();

            let tip_block_bytes = tx_db
                .get(self.get_tip_hash())
                .unwrap()
                .expect("The tip hash is not valid");
            let tip_block = Block::deserialize(tip_block_bytes.as_ref());
            if block.get_height() > tip_block.get_height() {
                let _ = tx_db.insert(TIP_BLOCK_HASH_KEY, block.get_hash()).unwrap();
                self.set_tip_hash(block.get_hash());
            }
            Ok(())
        });
    }

  
    pub fn get_best_height(&self) -> usize {
        let block_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        let tip_block_bytes = block_tree
            .get(self.get_tip_hash())
            .unwrap()
            .expect("The tip hash is valid");
        let tip_block = Block::deserialize(tip_block_bytes.as_ref());
        tip_block.get_height()
    }

  
    pub fn get_block(&self, block_hash: &[u8]) -> Option<Block> {
        let block_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        if let Some(block_bytes) = block_tree.get(block_hash).unwrap() {
            let block = Block::deserialize(block_bytes.as_ref());
            return Some(block);
        }
        return None;
    }

    
    pub fn get_block_hashes(&self) -> Vec<Vec<u8>> {
        let mut iterator = self.iterator();
        let mut blocks = vec![];
        loop {
            let option = iterator.next();
            if option.is_none() {
                break;
            }
            let block = option.unwrap();
            blocks.push(block.get_hash_bytes());
        }
        return blocks;
    }

    // Find all unspent transaction outputs (UTXOs) for a specific address
    // Returns a vector of transaction outputs that can be spent by this address
    // Used to calculate wallet balance and available funds for new transactions
    pub fn find_utxo_for_address(&self, pubkeyhash: &[u8]) -> Vec<TXOutput> {
        // Vector to store all unspent outputs for this address
        let mut utxos = Vec::new();
        // Map to track spent outputs
        let mut spent_txos: HashMap<String, Vec<usize>> = HashMap::new();

        // Create an iterator to traverse the blockchain
        let mut iterator = self.iterator();

        // Iterate through all blocks from newest to oldest
        loop {
            let option = iterator.next();
            if option.is_none() {
                break; // Reached the end of the blockchain
            }

            let block = option.unwrap();

            // For each transaction in the block
            for tx in block.get_transactions() {
                let txid_hex = HEXLOWER.encode(tx.get_id()); // Get transaction ID as hex

                // Check each output in the transaction
                for (idx, out) in tx.get_vout().iter().enumerate() {
                    // Skip if this output has been spent
                    if let Some(outs) = spent_txos.get(txid_hex.as_str()) {
                        let mut is_spent = false;

                        for spend_out_idx in outs {
                            if idx.eq(spend_out_idx) {
                                is_spent = true;
                                break;
                            }
                        }

                        if is_spent {
                            continue;
                        }
                    }

                    // If this output can be unlocked with the given public key hash (belongs to the address)
                    if out.is_locked_with_key(pubkeyhash) {
                        utxos.push(out.clone()); // Add to unspent outputs
                    }
                }

                // Skip coinbase transactions when tracking spent outputs
                if tx.is_coinbase() {
                    continue;
                }

                // For each input, mark the referenced output as spent
                for txin in tx.get_vin() {
                    let txid_hex = HEXLOWER.encode(txin.get_txid());

                    // Track this output as spent
                    if spent_txos.contains_key(txid_hex.as_str()) {
                        spent_txos
                            .get_mut(txid_hex.as_str())
                            .unwrap()
                            .push(txin.get_vout());
                    } else {
                        spent_txos.insert(txid_hex, vec![txin.get_vout()]);
                    }
                }
            }
        }

        utxos
    }

    // Find spendable outputs for a specific address up to a given amount
    // Used when creating a new transaction to determine which UTXOs to use
    pub fn find_spendable_outputs(
        &self,
        pubkeyhash: &[u8],
        amount: u64,
    ) -> (u64, HashMap<String, Vec<usize>>) {
        // Map to store transaction IDs and output indices that can be spent
        let mut unspent_outputs: HashMap<String, Vec<usize>> = HashMap::new();
        // Accumulator for the total amount found
        let mut accumulated = 0;
        // Map to track spent outputs
        let mut spent_txos: HashMap<String, Vec<usize>> = HashMap::new();

        // Create an iterator to traverse the blockchain
        let mut iterator = self.iterator();

        // Iterate through all blocks from newest to oldest
        'outer: loop {
            let option = iterator.next();
            if option.is_none() {
                break; // Reached the end of the blockchain
            }

            let block = option.unwrap();

            // For each transaction in the block
            for tx in block.get_transactions() {
                let txid_hex = HEXLOWER.encode(tx.get_id()); // Get transaction ID as hex

                // For each output in the transaction
                for (idx, out) in tx.get_vout().iter().enumerate() {
                    // Skip if this output has been spent
                    if let Some(outs) = spent_txos.get(txid_hex.as_str()) {
                        let mut is_spent = false;

                        for spend_out_idx in outs {
                            if idx.eq(spend_out_idx) {
                                is_spent = true;
                                break;
                            }
                        }

                        if is_spent {
                            continue;
                        }
                    }

                    // If this output can be unlocked with the given public key hash
                    if out.is_locked_with_key(pubkeyhash) {
                        // Add the value to our accumulated total
                        accumulated += out.get_value();
                        
                        // Add this output to our unspent outputs map
                        if unspent_outputs.contains_key(txid_hex.as_str()) {
                            unspent_outputs
                                .get_mut(txid_hex.as_str())
                                .unwrap()
                                .push(idx);
                        } else {
                            unspent_outputs.insert(txid_hex.clone(), vec![idx]);
                        }

                        // If we've found enough outputs to cover the amount, we can stop
                        if accumulated >= amount {
                            break 'outer;
                        }
                    }
                }

                // Skip coinbase transactions when tracking spent outputs
                if tx.is_coinbase() {
                    continue;
                }

                // For each input, mark the referenced output as spent
                for txin in tx.get_vin() {
                    let txid_hex = HEXLOWER.encode(txin.get_txid());

                    // Track this output as spent
                    if spent_txos.contains_key(txid_hex.as_str()) {
                        spent_txos
                            .get_mut(txid_hex.as_str())
                            .unwrap()
                            .push(txin.get_vout());
                    } else {
                        spent_txos.insert(txid_hex, vec![txin.get_vout()]);
                    }
                }
            }
        }

        // Return the total amount found and the map of spendable outputs
        (accumulated, unspent_outputs)
    }

    // Get a specific block by its hash
    pub fn get_block(&self, block_hash: &str) -> Block {
        // Open the blocks tree and retrieve the block
        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        // Deserialize the block from the database
        let data = blocks_tree.get(block_hash).unwrap().unwrap();
        
        // Convert the binary data back to a Block
        data.as_ref().into()
    }

    // Check if a block with the given hash exists in the blockchain
    pub fn block_exists(&self, block_hash: &str) -> bool {
        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        // Try to get the block - if it exists, return true
        blocks_tree.get(block_hash).unwrap().is_some()
    }

    // Get all the block hashes in the blockchain
    // Used for synchronizing with other nodes in the network
    pub fn get_block_hashes(&self) -> Vec<String> {
        // Vector to store all block hashes
        let mut block_hashes = Vec::new();
        // Create an iterator to traverse the blockchain
        let mut iterator = self.iterator();
        
        // Iterate through all blocks from newest to oldest
        loop {
            let option = iterator.next();
            if option.is_none() {
                break; // Reached the end of the blockchain
            }
            
            let block = option.unwrap();
            // Add this block's hash to the list
            block_hashes.push(String::from(block.get_hash()));
        }
        
        block_hashes
    }

    // Add a block to the blockchain
    // Used when receiving blocks from peers in the network
    pub fn add_block(&self, block: Block) {
        // Open the blocks tree
        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        // Get the block hash
        let block_hash = block.get_hash();
        
        // Check if we already have this block
        let exists = blocks_tree.get(block_hash).unwrap().is_some();
        
        if !exists {
            // Block doesn't exist - add it to the database
            let block_data = blocks_tree.get(block.get_pre_block_hash()).unwrap();
            if block_data.is_some() {
                // Update the database with the new block
                Self::update_blocks_tree(&blocks_tree, &block);
            }
        }
    }

    // Process new transactions in parallel and create a new block
    pub fn mine_block(&self, transactions: Vec<Transaction>) -> Block {
        // Process transactions in parallel
        let validated_txs = self.transaction_processor.process_pending_transactions();
        
        // Record block creation in PoH
        let (poh_hash, tick) = self.poh.record_event(self.get_tip_hash().as_bytes());
        
        // Create new block with PoH data
        let mut block = Block::new_block(
            validated_txs,
            self.get_tip_hash(),
            poh_hash,
            tick
        );
        
        // Update blockchain state
        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        Self::update_blocks_tree(&blocks_tree, &block);
        self.set_tip_hash(block.get_hash());
        
        block
    }

    // Add a new transaction to the pending pool
    pub fn add_transaction(&self, transaction: Transaction) {
        self.transaction_processor.add_transaction(transaction);
    }

    // Get current transactions per second
    pub fn get_tps(&self) -> f64 {
        self.poh.get_tps(0, 0) // You'll want to track these values properly
    }
}

// Iterator for traversing the blockchain from newest to oldest block
pub struct BlockchainIterator {
    current_hash: String, // The hash of the current block
    db: Db,              // Database connection
}

impl BlockchainIterator {
    // Create a new blockchain iterator starting from the given hash
    // This is typically initialized with the tip (newest block) hash
    fn new(current_hash: String, db: Db) -> Self {
        BlockchainIterator { current_hash, db }
    }

    // Get the next block in the chain (moving from newest to oldest)
    // Returns None when the end of the blockchain is reached (after genesis block)
    pub fn next(&mut self) -> Option<Block> {
        // Open the blocks tree
        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        // Get the current block data from the database using its hash
        let encoded_block = blocks_tree.get(self.current_hash.as_str()).unwrap();
        
        if encoded_block.is_none() {
            return None; // No more blocks (shouldn't happen with valid blockchain)
        }
        
        // Deserialize the binary data into a Block object
        let block: Block = encoded_block.unwrap().as_ref().into();
        // Update the current hash to the previous block's hash
        // This advances the iterator to the older block in the next call
        self.current_hash = String::from(block.get_pre_block_hash());
        
        // Return the current block
        Some(block)
    }
}


