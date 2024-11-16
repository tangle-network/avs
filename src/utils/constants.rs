use alloy_primitives::{address, Address};

pub mod local {
    use super::*;
    pub const AVS_DIRECTORY_ADDR: Address = address!("0165878A594ca255338adfa4d48449f69242Eb8F");
    pub const DELEGATION_MANAGER_ADDR: Address =
        address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
    pub const ERC20_MOCK_ADDR: Address = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");
    pub const OPERATOR_STATE_RETRIEVER_ADDR: Address =
        address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
    pub const REGISTRY_COORDINATOR_ADDR: Address =
        address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
    pub const SERVICE_MANAGER_ADDR: Address = address!("67d269191c92caf3cd7723f116c85e6e9bf55933");
    pub const STRATEGY_MANAGER_ADDR: Address = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");
    pub const ZERO_ADDRESS: Address = address!("0000000000000000000000000000000000000000");
}

pub mod holesky {
    use super::*;
    pub const TANGLE_SERVICE_MANAGER_ADDR: Address =
        address!("5aBc6138DD384a1b059f1fcBaD73E03c31170C14");
    pub const ECDSA_STAKE_REGISTRY_ADDR: Address =
        address!("131b803Bece581281A2E33d7E693DfA70aB85D06");
    pub const AVS_DIRECTORY_ADDR: Address = address!("055733000064333CaDDbC92763c58BF0192fFeBf");
    pub const DELEGATION_MANAGER_ADDR: Address =
        address!("A44151489861Fe9e3055d95adC98FbD462B948e7");
    pub const STRATEGY_MANAGER_ADDR: Address = address!("dfB5f6CE42aAA7830E94ECFCcAd411beF4d4D5b6");
    pub const ZERO_ADDRESS: Address = address!("0000000000000000000000000000000000000000");
}
