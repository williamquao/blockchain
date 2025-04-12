# Rust Blockchain

A feature-complete blockchain implementation in Rust, featuring transactions, wallets, mining, and a peer-to-peer network.

## Features

- Basic blockchain data structure with Proof of Work consensus
- Persistent storage using the Sled database
- UTXO (Unspent Transaction Output) model for transaction handling
- Cryptographically secure wallets using ECDSA
- Mining with rewards
- Peer-to-peer networking between nodes
- Command-line interface for all blockchain operations

## Prerequisites

- Rust and Cargo (latest stable version)
- Git (for cloning the repository)

## Installation

1. Clone this repository:

   ```
   git clone https://github.com/yourusername/blockchain_rust.git
   cd blockchain_rust
   ```

2. Build the project:

   ```
   cargo build --release
   ```

3. Run the binary (instead of using `cargo run`):
   ```
   ./target/release/blockchain_rust --help
   ```

## Usage

### Managing Wallets

#### Create a New Wallet

```bash
cargo run -- createwallet
```

This will generate a new wallet and print its Base58-encoded address.

#### List All Wallet Addresses

```bash
cargo run -- listaddresses
```

This will display all of your wallet addresses stored in the wallet.dat file.

#### Check Wallet Balance

```bash
cargo run -- getbalance ADDRESS
```

Replace `ADDRESS` with your Base58-encoded wallet address.

### Blockchain Operations

#### Create a New Blockchain

```bash
cargo run -- createblockchain ADDRESS
```

This creates a new blockchain and sends the mining reward of the genesis block to the specified address.

#### Print Blockchain Information

```bash
cargo run -- printchain
```

Shows detailed information about each block in the blockchain, including transactions.

#### Rebuild UTXO Set

```bash
cargo run -- reindexutxo
```

Rebuilds the UTXO (Unspent Transaction Output) set, which is used to track spendable outputs.

### Transactions

#### Send Coins

```bash
cargo run -- send FROM_ADDRESS TO_ADDRESS AMOUNT MINE
```

Parameters:

- `FROM_ADDRESS`: The sender's wallet address
- `TO_ADDRESS`: The recipient's wallet address
- `AMOUNT`: The amount to send
- `MINE`: Set to 1 to mine the transaction immediately on this node, or 0 to broadcast to network

### Networking

#### Start a Node

```bash
cargo run -- startnode
```

Starts a blockchain node that can communicate with other nodes in the network.

#### Start a Mining Node

```bash
cargo run -- startnode --miner ADDRESS
```

Starts a node in mining mode, which will automatically mine new blocks when transactions are received. Mining rewards will be sent to the specified address.

## Architecture

The blockchain is composed of several core components:

1. **Block**: The fundamental unit of the blockchain, containing transaction data and linking to the previous block.

2. **Blockchain**: Manages the chain of blocks, handles the addition of new blocks, and provides methods to query the blockchain data.

3. **Transaction**: Represents a transfer of value between wallets, consisting of inputs and outputs.

4. **UTXO Set**: An optimized data structure for tracking unspent transaction outputs, improving the efficiency of balance inquiries.

5. **Wallet**: Manages cryptographic keys and provides an address for receiving funds.

6. **Node**: Handles peer-to-peer communication and network synchronization.

7. **Proof of Work**: The consensus mechanism that ensures the security and integrity of the blockchain.

## Technical Details

- **Language**: Rust 2021 Edition
- **Cryptography**: ECDSA with P-256 curve and SHA-256 for signatures
- **Addressing**: Base58-encoded addresses with version byte and checksum (similar to Bitcoin)
- **Database**: Sled embedded database for blockchain storage
- **Serialization**: Bincode for efficient binary serialization
- **Network Protocol**: Custom protocol over TCP for peer-to-peer communication

## Project Structure

- `src/block.rs`: Block structure and proof of work implementation
- `src/blockchain.rs`: Blockchain data structure and operations
- `src/transactions.rs`: Transaction structures and validation
- `src/utxoSet.rs`: UTXO set implementation for efficient transaction processing
- `src/wallets.rs`: Wallet management for key generation and address handling
- `src/server.rs`: Network server for node communication
- `src/node.rs`: Node management and peer discovery
- `src/utils.rs`: Utility functions for cryptography and encoding
- `src/config.rs`: Configuration settings for the blockchain
- `src/memory_pool.rs`: Temporary storage for unconfirmed transactions
- `src/main.rs`: Command-line interface implementation

## License

[MIT License](LICENSE)

## Acknowledgments

This project is inspired by various blockchain implementations and educational resources, including:

- Bitcoin's design principles
- Ethereum's account model (parts of it)
- Various blockchain learning resources and tutorials

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the project
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
# blockchain
