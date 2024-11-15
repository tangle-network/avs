#!/bin/bash

# Set RPC URL
RPC_URL="https://ethereum-holesky.publicnode.com"

# Function to verify a proxy contract
verify_proxy() {
    local contract_name=$1
    local proxy_address=$2
    local implementation_address=$3
    local contract_path=$4
    
    echo "Verifying $contract_name..."
    echo "Proxy Address: $proxy_address"
    echo "Implementation Address: $implementation_address"
    echo "Path: $contract_path"
    echo "----------------------------------------"
    
    # First verify the implementation contract
    forge verify-contract $implementation_address $contract_path --rpc-url $RPC_URL --watch --via-ir --compiler-version "v0.8.12+commit.f00d7308" --optimizer-runs 200
    
    # Then verify the proxy contract
    forge verify-contract $proxy_address contracts/lib/eigenlayer-middleware/lib/openzeppelin-contracts/contracts/proxy/transparent/TransparentUpgradeableProxy.sol:TransparentUpgradeableProxy --rpc-url $RPC_URL --watch --via-ir --compiler-version "v0.8.12+commit.f00d7308" --optimizer-runs 200
    
    if [ $? -eq 0 ]; then
        echo "✅ $contract_name verification submitted successfully"
    else
        echo "❌ $contract_name verification failed"
    fi
    echo "----------------------------------------"
    sleep 2
}

# Function to verify a regular contract
verify_contract() {
    local contract_name=$1
    local contract_address=$2
    local contract_path=$3
    
    echo "Verifying $contract_name..."
    echo "Address: $contract_address"
    echo "Path: $contract_path"
    echo "----------------------------------------"
    
    forge verify-contract $contract_address $contract_path --rpc-url $RPC_URL --watch --via-ir --compiler-version "v0.8.12+commit.f00d7308" --optimizer-runs 200
    
    if [ $? -eq 0 ]; then
        echo "✅ $contract_name verification submitted successfully"
    else
        echo "❌ $contract_name verification failed"
    fi
    echo "----------------------------------------"
    sleep 2
}

# Create a log directory if it doesn't exist
mkdir -p logs

# Redirect all output to both console and log file
exec > >(tee -a "logs/verification_$(date +%Y%m%d_%H%M%S).log") 2>&1

echo "Starting contract verification process..."
echo "Time: $(date)"
echo "=========================================="

# First verify the non-proxy contracts
verify_contract "PauserRegistry" \
    "0x06399b7f1Bc83942F44e6E84c44bd50A39A98d4a" \
    "contracts/lib/eigenlayer-middleware/lib/eigenlayer-contracts/src/contracts/permissions/PauserRegistry.sol:PauserRegistry"

verify_contract "ProxyAdmin" \
    "0x73dfBAB6836B466f66126b749eA83581c021d203" \
    "contracts/lib/eigenlayer-middleware/lib/openzeppelin-contracts/contracts/proxy/transparent/ProxyAdmin.sol:ProxyAdmin"

verify_contract "OperatorStateRetriever" \
    "0x38F984394c123375ACb7A638b66a78e2D15c987b" \
    "contracts/lib/eigenlayer-middleware/src/OperatorStateRetriever.sol:OperatorStateRetriever"

verify_contract "EmptyContract" \
    "0x9547D0e9Aa14Ce946D3EA92e48307069f3F2C0a9" \
    "contracts/lib/eigenlayer-middleware/lib/eigenlayer-contracts/src/test/mocks/EmptyContract.sol:EmptyContract"

# Then verify the proxy contracts and their implementations
# RegistryCoordinator
verify_proxy "RegistryCoordinator" \
    "0x227327316CA7Ec350bb8651bE940C005f3B50c78" \
    "0x0ebA2dD3ddeB33E7B0F362265DeA4d4ed6FB77Fd" \
    "contracts/lib/eigenlayer-middleware/src/RegistryCoordinator.sol:RegistryCoordinator"

# BLSApkRegistry
verify_proxy "BLSApkRegistry" \
    "0x132A0B5525170a5e7ca55C9933f4ED501519B229" \
    "0xbE5b158e847e53ff0F76fb89d23a115e45956D67" \
    "contracts/lib/eigenlayer-middleware/src/BLSApkRegistry.sol:BLSApkRegistry"

# IndexRegistry
verify_proxy "IndexRegistry" \
    "0xDfDc051dF13b3437488a5d29C99A7f8c544a152F" \
    "0x65D92d531C375EDf4f2Da31e97308B9fa3554B20" \
    "contracts/lib/eigenlayer-middleware/src/IndexRegistry.sol:IndexRegistry"

# StakeRegistry
verify_proxy "StakeRegistry" \
    "0x8Ab6DccB5768f8Da704e1c88a1D9E69e4b67C452" \
    "0x60DaAa39914E086686A9A77a3dA176a33da93c7a" \
    "contracts/lib/eigenlayer-middleware/src/StakeRegistry.sol:StakeRegistry"

# TangleServiceManager
verify_proxy "TangleServiceManager" \
    "0x2ad7E1CDd225eD5FD86C9cAe60B965d434C02660" \
    "0x4b32ce8bC7d0659830a9B18b84A6CEf3cFA99D5A" \
    "contracts/src/TangleServiceManager.sol:TangleServiceManager"

echo "=========================================="
echo "Verification process completed"
echo "Time: $(date)"
echo "Please check the log file for detailed results"
