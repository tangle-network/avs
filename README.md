# <h1 align="center"> Tangle AVS on Eigenlayer 🌐 </h1>

## 📚 Overview

The Tangle AVS (Actively Validated Service) on Eigenlayer allows operators to run either a Tangle validator or operator, contributing to the Tangle Network's security and decentralization while earning rewards through the Eigenlayer ecosystem.

By participating in the Tangle AVS, operators and restakers will be eligible for points on the Tangle Network. These points will be convertible to tokens through an airdrop when Tangle launches on Eigenlayer's testnet and mainnet.

## 🚀 Features

- Run a Tangle validator or operator
- Earn points convertible to Tangle tokens
- Contribute to Tangle Network's security and decentralization
- Leverage Eigenlayer's restaking mechanism

## 🛠️ Getting Started

### Prerequisites

Before you can run this project, you will need to have the following software installed on your machine:

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Node.js and npm](https://nodejs.org/)
- [Docker](https://www.docker.com/get-started) (optional, for containerized deployment)

### Installation

1. Clone the repository:
   ```
   git clone https://github.com/your-repo/tangle-avs-eigenlayer.git
   cd tangle-avs-eigenlayer
   ```

2. Install dependencies:
   ```
   cargo build --release
   ```

### Configuration

1. Set up your Eigenlayer credentials and endpoints in the `.env` file:
   ```
   OPERATOR_PRIVATE_KEY=<your_private_key>
   ```

2. Choose your role (validator or operator) in the `config.toml` file:
   ```toml
   [tangle]
   role = "validator" # or "operator" or "both"
   ```

### Running the AVS

1. Start the Tangle AVS:
   ```
   cargo run --release
   ```

2. Monitor the logs for successful registration and operation.

## 💰 Rewards and Points

- Operators and restakers earn points for their participation in the Tangle AVS.
- Points will be convertible to Tangle tokens through an airdrop at a later date.
- The exact conversion rate and airdrop details will be announced closer to the Tangle Network launch on Eigenlayer.

## 🤝 Support and Community

- Follow us on [Twitter](https://twitter.com/tangle_network) for the latest news.
- Check our [Documentation](https://docs.tangle.tools) for detailed guides and FAQs.
- If you have any feedback or issues, please feel free to open an issue on our [GitHub repository](https://github.com/webb-tools/blueprint-template/issues).

## 📜 License

This project is licensed under the Unlicense License. See the [LICENSE](./LICENSE) file for more details.
