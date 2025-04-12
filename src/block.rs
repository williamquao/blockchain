// --------------------------------------------------------------------------------------------------
// Getting started with building the Blockchain ( block.rs / proof of work.rs)
// --------------------------------------------------------------------------------------------------

// Import necessary dependencies
use crate::Transaction; // Import Transaction struct from the crate root
use serde::{Deserialize, Serialize}; // For serializing/deserializing blocks
use sled::IVec; // For database storage compatibility
use data_encoding::HEXLOWER; // For encoding binary data as lowercase hexadecimal strings
use num_bigint::{BigInt, Sign}; // For large integer operations used in Proof of Work
use std::borrow::Borrow; // For borrowing references efficiently
use std::ops::ShlAssign; // For bit shift operations
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

// The Block struct - fundamental unit of the blockchain
// Derive Clone for copying, Serialize/Deserialize for converting to/from binary
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    timestamp: u64,
    transactions: Vec<Transaction>,
    pre_block_hash: String,
    hash: String,
    height: u64,
    poh_hash: Vec<u8>,    // Proof of History hash
    poh_tick: u64,        // PoH tick count
    merkle_root: String,  // Merkle root of transactions
}

impl Block {
    // Creates a new block with given previous hash, transactions, and height
    pub fn new_block(
        transactions: Vec<Transaction>,
        pre_block_hash: String,
        poh_hash: Vec<u8>,
        poh_tick: u64,
    ) -> Block {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let merkle_root = Self::calculate_merkle_root(&transactions);
        
        let mut block = Block {
            timestamp,
            transactions,
            pre_block_hash,
            hash: String::new(),
            height: 0,
            poh_hash,
            poh_tick,
            merkle_root,
        };

        block.hash = block.calculate_hash();
        block
    }

    // Calculate Merkle root of transactions for efficient verification
    fn calculate_merkle_root(transactions: &[Transaction]) -> String {
        if transactions.is_empty() {
            return String::from("0000000000000000000000000000000000000000000000000000000000000000");
        }

        let mut hashes: Vec<String> = transactions
            .iter()
            .map(|tx| tx.calculate_hash())
            .collect();

        while hashes.len() > 1 {
            let mut new_hashes = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(chunk[0].as_bytes());
                if chunk.len() > 1 {
                    hasher.update(chunk[1].as_bytes());
                } else {
                    hasher.update(chunk[0].as_bytes()); // Duplicate last hash if odd number
                }
                new_hashes.push(format!("{:x}", hasher.finalize()));
            }
            hashes = new_hashes;
        }

        hashes[0].clone()
    }

    // Verify block's PoH data
    pub fn verify_poh(&self, previous_poh_hash: &[u8], previous_tick: u64) -> bool {
        self.poh_tick > previous_tick && 
        self.poh_hash != previous_poh_hash
    }

    // Get block's PoH hash
    pub fn get_poh_hash(&self) -> &[u8] {
        &self.poh_hash
    }

    // Get block's PoH tick
    pub fn get_poh_tick(&self) -> u64 {
        self.poh_tick
    }

    // Get block's Merkle root
    pub fn get_merkle_root(&self) -> &str {
        &self.merkle_root
    }

    // Deserialize a block from its binary representation
    pub fn deserialize(bytes: &[u8]) -> Block {
        bincode::deserialize(bytes).unwrap() // Convert binary data back to Block struct
    }

    // Serialize this block into a binary representation for storage
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap().to_vec() // Convert Block struct to binary
    }

    // Create the first block in the blockchain (genesis block)
    pub fn generate_genesis_block(transaction: &Transaction) -> Block {
        // Genesis block contains a coinbase transaction (mining reward)
        let transactions = vec![transaction.clone()];
        // Create a new block with "None" as previous hash, the coinbase transaction, and height 0
        return Block::new_block(transactions, String::from("None"), vec![], 0);
    }

    // Calculate a hash of all transactions in this block
    // Used in the mining process and for verification
    pub fn hash_transactions(&self) -> Vec<u8> {
        let mut txhashs = vec![]; // Vector to store concatenated transaction IDs
        for transaction in &self.transactions {
            txhashs.extend(transaction.get_id()); // Add each transaction's ID
        }
        // Hash the combined transaction IDs using SHA-256
        crate::sha256_digest(txhashs.as_slice())
    }

    // Getter for block transactions
    pub fn get_transactions(&self) -> &[Transaction] {
        self.transactions.as_slice() // Return transactions as a slice
    }

    // Getter for previous block hash
    pub fn get_pre_block_hash(&self) -> String {
        self.pre_block_hash.clone() // Return a clone of the previous block hash
    }

    // Getter for this block's hash
    pub fn get_hash(&self) -> &str {
        self.hash.as_str() // Return hash as a string slice
    }

    // Get block hash as a byte vector
    pub fn get_hash_bytes(&self) -> Vec<u8> {
        self.hash.as_bytes().to_vec() // Convert hash string to bytes
    }

    // Getter for block timestamp
    pub fn get_timestamp(&self) -> u64 {
        self.timestamp // Return the timestamp
    }

    // Getter for block height
    pub fn get_height(&self) -> u64 {
        self.height // Return the height
    }
}

// Implement From trait to convert Block to IVec for database storage
impl From<Block> for IVec {
    fn from(b: Block) -> Self {
        let bytes = bincode::serialize(&b).unwrap(); // Serialize block to binary
        Self::from(bytes) // Convert to IVec (sled database type)
    }
}

// Proof of Work implementation - the consensus algorithm
pub struct ProofOfWork {
    block: Block,  // The block to mine
    target: BigInt, // The target value that a valid hash must be below
}

// Difficulty of mining - higher value = easier mining, lower value = harder
// In Bitcoin this is dynamic, here it's fixed for simplicity
const TARGET_BITS: i32 = 8;

// Maximum value for nonce before resetting other block parameters
const MAX_NONCE: i64 = i64::MAX;

impl ProofOfWork {
    // Create a new Proof of Work instance for a given block
    pub fn new_proof_of_work(block: Block) -> ProofOfWork {
        let mut target = BigInt::from(1); // Start with 1
       
        // Create target value: 1 shifted left by (256-TARGET_BITS)
        // This creates a number with many leading zeros when expressed in binary
        // The more leading zeros required, the harder the mining process
        target.shl_assign(256 - TARGET_BITS);
        ProofOfWork { block, target }
    }

    // Prepare the data to be hashed during mining
    // This combines block data with a nonce for each attempt
    fn prepare_data(&self, nonce: i64) -> Vec<u8> {
        let pre_block_hash = self.block.get_pre_block_hash(); // Get previous block hash
        let transactions_hash = self.block.hash_transactions(); // Calculate transactions hash
        let timestamp = self.block.get_timestamp(); // Get block timestamp
        
        // Combine all data into a single byte vector
        let mut data_bytes = vec![];
        data_bytes.extend(pre_block_hash.as_bytes()); // Add previous block hash
        data_bytes.extend(transactions_hash);         // Add transactions hash
        data_bytes.extend(timestamp.to_be_bytes());   // Add timestamp
        data_bytes.extend(TARGET_BITS.to_be_bytes()); // Add target bits
        data_bytes.extend(nonce.to_be_bytes());       // Add current nonce attempt
        return data_bytes;
    }

    // Run the mining process to find a valid nonce and hash
    pub fn run(&self) -> (i64, String) {
        let mut nonce = 0; // Start with nonce = 0
        let mut hash = Vec::new(); // Initialize empty hash
        
        println!("Mining the block"); // Logging
        
        // Try different nonce values until finding a valid one or reaching the maximum
        while nonce < MAX_NONCE {
            // Prepare data for this nonce attempt
            let data = self.prepare_data(nonce);
            // Calculate SHA-256 hash of the data
            hash = crate::sha256_digest(data.as_slice());
            // Convert hash bytes to a BigInt for comparison with target
            let hash_int = BigInt::from_bytes_be(Sign::Plus, hash.as_slice());

            // Check if hash is less than target (valid block found)
            if hash_int.lt(self.target.borrow()) {
                // Print hash and exit loop on success
                println!("{}", HEXLOWER.encode(hash.as_slice()));
                break;
            } else {
                // Try next nonce value
                nonce += 1;
            }
        }
        
        println!(); // Spacing in output
        
        // Return the successful nonce and hash as a hex string
        return (nonce, HEXLOWER.encode(hash.as_slice()));
    }
}
