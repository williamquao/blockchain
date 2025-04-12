// Wallet implementation for the blockchain
// Handles cryptographic keys and Bitcoin-style addresses
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use serde::{Deserialize, Serialize};

// Version byte for addresses (similar to Bitcoin mainnet addresses which start with 1)
const VERSION: u8 = 0x00;
// Length of the checksum used in addresses (last 4 bytes)
pub const ADDRESS_CHECK_SUM_LEN: usize = 4;

// Wallet struct - stores the cryptographic identity of a user
// Contains private and public keys used for signing transactions
#[derive(Clone, Serialize, Deserialize)]
pub struct Wallet {
    pkcs8: Vec<u8>,      // Private key in PKCS#8 format (secured, never shared)
    public_key: Vec<u8>, // Public key (used for verification, can be shared)
}

impl Wallet {
    
    // Create a new wallet with a fresh key pair
    pub fn new() -> Wallet {
        // Generate a new private key
        let pkcs8 = crate::new_key_pair();
        // Create an ECDSA key pair from the private key
        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref()).unwrap();
        // Extract the public key
        let public_key = key_pair.public_key().as_ref().to_vec();
        // Create and return the wallet
        Wallet { pkcs8, public_key }
    }

    // Generate a Bitcoin-style address from the wallet's public key
    // This is what users share to receive funds
    pub fn get_address(&self) -> String {
        // Hash the public key (SHA-256 + RIPEMD-160)
        let pub_key_hash = hash_pub_key(self.public_key.as_slice());
        
        // Create the address payload
        let mut payload: Vec<u8> = vec![];
        payload.push(VERSION);                  // Add version byte
        payload.extend(pub_key_hash.as_slice()); // Add hashed public key
        
        // Calculate checksum (first 4 bytes of double SHA-256)
        let checksum = checksum(payload.as_slice());
        payload.extend(checksum.as_slice());    // Add checksum to payload
        
        // Encode the complete payload as Base58
        crate::base58_encode(payload.as_slice())
    }

    // Get the wallet's public key
    pub fn get_public_key(&self) -> &[u8] {
        self.public_key.as_slice()
    }

    // Get the wallet's private key (PKCS#8 format)
    // This should be used carefully as it gives full access to the wallet
    pub fn get_pkcs8(&self) -> &[u8] {
        self.pkcs8.as_slice()
    }
}

// Hash a public key to create the core of an address
// Uses SHA-256 followed by RIPEMD-160 (same as Bitcoin)
pub fn hash_pub_key(pub_key: &[u8]) -> Vec<u8> {
    // First hash with SHA-256
    let pub_key_sha256 = crate::sha256_digest(pub_key);
    // Then hash the result with RIPEMD-160
    crate::ripemd160_digest(pub_key_sha256.as_slice())
}

// Calculate the checksum for an address
// Takes a version+hash payload and returns the first 4 bytes of double-SHA256
fn checksum(payload: &[u8]) -> Vec<u8> {
    // First round of SHA-256
    let first_sha = crate::sha256_digest(payload);
    // Second round of SHA-256
    let second_sha = crate::sha256_digest(first_sha.as_slice());
    // Return first 4 bytes as checksum
    second_sha[0..ADDRESS_CHECK_SUM_LEN].to_vec()
}

// Validate if an address is properly formatted and has a valid checksum
pub fn validate_address(address: &str) -> bool {
    // Decode the Base58 address
    let payload = crate::base58_decode(address);
    
    // Extract the checksum from the address
    let actual_checksum = payload[payload.len() - ADDRESS_CHECK_SUM_LEN..].to_vec();
    
    // Extract the version and pub key hash
    let version = payload[0];
    let pub_key_hash = payload[1..payload.len() - ADDRESS_CHECK_SUM_LEN].to_vec();

    // Recalculate the checksum
    let mut target_vec = vec![];
    target_vec.push(version);
    target_vec.extend(pub_key_hash);
    let target_checksum = checksum(target_vec.as_slice());
    
    // Compare checksums to validate the address
    actual_checksum.eq(target_checksum.as_slice())
}

// Convert a public key hash directly to an address
// Useful for looking up addresses from transaction outputs
pub fn convert_address(pub_hash_key: &[u8]) -> String {
    // Create address payload
    let mut payload: Vec<u8> = vec![];
    payload.push(VERSION);              // Add version byte
    payload.extend(pub_hash_key);       // Add public key hash
    
    // Calculate and add checksum
    let checksum = checksum(payload.as_slice());
    payload.extend(checksum.as_slice());
    
    // Encode as Base58
    crate::base58_encode(payload.as_slice())
}

// Wallets manager - stores and manages multiple wallets
// Additional imports needed for file operations
use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};

// File name for storing wallets on disk
pub const WALLET_FILE: &str = "wallet.dat";

// Wallets struct - manages multiple wallets in memory and on disk
pub struct Wallets {
    wallets: HashMap<String, Wallet>, // Maps addresses to wallets
}

impl Wallets {
    // Create a new Wallets instance and load existing wallets from disk
    pub fn new() -> Wallets {
        let mut wallets = Wallets {
            wallets: HashMap::new(),
        };
        // Try to load existing wallets from file
        wallets.load_from_file();
        return wallets;
    }

    // Create a new wallet, save it, and return its address
    pub fn create_wallet(&mut self) -> String {
        // Generate a new wallet
        let wallet = Wallet::new();
        // Get its address
        let address = wallet.get_address();
        // Store the wallet in memory
        self.wallets.insert(address.clone(), wallet);
        // Save all wallets to disk
        self.save_to_file();
        // Return the new address
        return address;
    }

    // Get a list of all wallet addresses
    pub fn get_addresses(&self) -> Vec<String> {
        let mut addresses = vec![];
        // Collect all addresses from the wallets map
        for (address, _) in &self.wallets {
            addresses.push(address.clone())
        }
        return addresses;
    }

    // Get a specific wallet by its address
    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        // Look up the wallet in the map
        if let Some(wallet) = self.wallets.get(address) {
            return Some(wallet);
        }
        None
    }

    // Load wallets from the wallet file on disk
    pub fn load_from_file(&mut self) {
        // Determine file path
        let path = current_dir().unwrap().join(WALLET_FILE);
        // If file doesn't exist, just return (no wallets to load)
        if !path.exists() {
            return;
        }
        
        // Open and read the wallet file
        let mut file = File::open(path).unwrap();
        let metadata = file.metadata().expect("unable to read metadata");
        let mut buf = vec![0; metadata.len() as usize];
        let _ = file.read(&mut buf).expect("buffer overflow");
        
        // Deserialize the wallet data
        let wallets = bincode::deserialize(&buf[..]).expect("unable to deserialize file data");
        self.wallets = wallets;
    }

    // Save wallets to disk
    fn save_to_file(&self) {
        // Determine file path
        let path = current_dir().unwrap().join(WALLET_FILE);
        
        // Open the file for writing (create if it doesn't exist)
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path)
            .expect("unable to open wallet.dat");
            
        // Create a buffered writer
        let mut writer = BufWriter::new(file);
        
        // Serialize the wallets
        let wallets_bytes = bincode::serialize(&self.wallets).expect("unable to serialize wallets");
        
        // Write to file
        writer.write(wallets_bytes.as_slice()).unwrap();
        let _ = writer.flush();
    }
}

