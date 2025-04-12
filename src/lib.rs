// Import the block module which contains the Block structure and Proof of Work implementation
mod block;
// Import the Block struct from the block module to make it available in this scope
use block::Block;

// Import the blockchain module which contains the blockchain data structure and operations
mod blockchain;
// Re-export the Blockchain struct to make it publicly available to users of this crate
pub use blockchain::Blockchain;

// Import the UTXO (Unspent Transaction Output) set module, which tracks unspent outputs for efficiency
mod utxo_set;
// Re-export the UTXOSet struct for public use
pub use utxo_set::UTXOSet;

// Import the transactions module which defines the transaction structure and logic
mod transactions;
// Re-export the Transaction struct and its related components for public use
pub use transactions::Transaction;
pub use transactions::TXInput;
pub use transactions::TXOutput;

// Import the wallets module and make it public so users can access wallet functionality directly
pub mod wallets;
// Re-export wallet-related elements for public use
pub use wallets::Wallet;
pub use wallets::ADDRESS_CHECK_SUM_LEN; // Constant used in wallet address validation
pub use wallets::hash_pub_key; // Function for hashing public keys to generate addresses

// Import the server module which handles network communication
mod server;
// Re-export server-related elements for public use
pub use server::send_tx; // Function for sending transactions over the network
pub use server::Package; // Enum for network message types
pub use server::Server; // Main server struct for node communication
pub use server::CENTERAL_NODE; // Constant defining the central node address

// Import the node module which manages peer nodes in the network
mod node;
// Re-export the Nodes struct which maintains a list of network peers
pub use node::Nodes;

// Import the memory_pool module which handles unconfirmed transactions
mod memory_pool;
// Re-export memory pool components
pub use memory_pool::BlockInTransit; // Struct for blocks being synchronized between nodes
pub use memory_pool::MemoryPool; // Struct for temporary storage of pending transactions

// Import the configuration module
mod config;
// Re-export configuration elements
pub use config::Config; // Configuration struct
pub use config::GLOBAL_CONFIG; // Global configuration singleton

// Import the utils module and make it public for general utility functions
pub mod utils;
// Import specific utility functions for use within this crate
use utils::base58_decode; // Function for decoding Base58 strings (used for addresses)
use utils::base58_encode; // Function for encoding data as Base58 strings
use utils::current_timestamp; // Function for getting the current time in milliseconds
use utils::ecdsa_p256_sha256_sign_digest; // Function for signing data with ECDSA
use utils::ecdsa_p256_sha256_sign_verify; // Function for verifying ECDSA signatures
use utils::new_key_pair; // Function for generating new cryptographic key pairs
use utils::ripemd160_digest; // Function for RIPEMD-160 hashing (used in address generation)
use utils::sha256_digest; // Function for SHA-256 hashing (used throughout the blockchain)
