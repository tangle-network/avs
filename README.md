# <h1 align="center"> Tangle AVS on Eigenlayer 🌐 </h1>

## 📚 Overview

The Tangle AVS (Actively Validated Service) on Eigenlayer enables operators to run either a Tangle validator or operator node, contributing to the Tangle Network's security and decentralization while earning rewards through the Eigenlayer ecosystem.

By participating in the Tangle AVS, operators and restakers will be eligible for points on the Tangle Network. These points will be convertible to tokens through an airdrop when Tangle launches on Eigenlayer's testnet and mainnet.

## Architecture

![Tangle AVS Architecture](asset/architecture.png)

## 🚀 Key Features

- Deploy and manage a Tangle validator or operator node
- Accumulate points redeemable for Tangle tokens
- Strengthen Tangle Network's security and decentralization
- Utilize Eigenlayer's innovative restaking mechanism

## 🛠️ Getting Started

### Prerequisites

To run this project, ensure you have the following software installed on your system:

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Node.js and npm](https://nodejs.org/)
- [Docker](https://www.docker.com/get-started) (optional, for containerized deployment)

For local testing, you'll also need:

- [Anvil](https://book.getfoundry.sh/getting-started/installation) (optional, for running a local testnet)

### Installation

1. Clone the repository:
   ```
   git clone https://github.com/tangle-network/avs.git tangle-avs
   cd tangle-avs
   ```

2. Install dependencies:
   ```
   cargo build --release
   ```

### Configuration

1. Configure your Eigenlayer credentials and endpoints in the `.env` file:
   ```
   OPERATOR_PRIVATE_KEY=<your_private_key>
   ```

2. Specify your role (validator, operator, or both) in the `config.toml` file:
   ```toml
   [tangle]
   role = "validator" # or "operator" or "both"
   ```

### Launching the AVS

1. Initiate the Tangle AVS:
   ```
   cargo run --release
   ```

2. Monitor the logs to ensure successful registration and operation.

## 💰 Rewards and Points

- Operators and restakers accumulate points for their participation in the Tangle AVS.
- These points will be convertible to Tangle tokens through a future airdrop.
- The precise conversion rate and airdrop details will be announced closer to the Tangle Network's launch on Eigenlayer.

## 🤝 Community and Support

- Stay updated by following us on [Twitter](https://twitter.com/tangle_network).
- Explore our comprehensive [Documentation](https://docs.tangle.tools) for detailed guides and FAQs.
- For feedback, questions, or issues, please open an issue on our [GitHub repository](https://github.com/webb-tools/blueprint-template/issues).

## 📜 License

This project is released under the Unlicense License. For more information, refer to the [LICENSE](./LICENSE) file.