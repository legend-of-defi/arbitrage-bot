#!/bin/bash
# Fetch latest block numbers for Base and Ethereum networks

# Get Public Ethereum latest block
ETH_BLOCK=$(curl -s -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' https://ethereum-rpc.publicnode.com | jq -r '.result' | xargs printf "%d")

# Get Public Base network latest block
BASE_BLOCK=$(curl -s -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' https://mainnet.base.org | jq -r '.result' | xargs printf "%d")

# Get Local Ethereum latest block - For WebSocket, we should find corresponding HTTP endpoint
# Try both HTTP and WebSocket ports
if command -v websocat >/dev/null 2>&1; then
  # WebSocket client available
  LOCAL_ETH_BLOCK=$(echo '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' | websocat -n1 ws://localhost:8646 2>/dev/null | jq -r '.result' 2>/dev/null | xargs printf "%d" 2>/dev/null || echo "N/A")
else
  # Try HTTP on same port (common setup)
  LOCAL_ETH_BLOCK=$(curl -s -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' http://localhost:8645 2>/dev/null | jq -r '.result' 2>/dev/null | xargs printf "%d" 2>/dev/null || echo "N/A")

  # If that fails, try the WebSocket port but with HTTP
  if [ "$LOCAL_ETH_BLOCK" = "N/A" ]; then
    LOCAL_ETH_BLOCK=$(curl -s -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' http://localhost:8646 2>/dev/null | jq -r '.result' 2>/dev/null | xargs printf "%d" 2>/dev/null || echo "N/A")
  fi
fi

# Get Local Base latest block - For WebSocket, we should find corresponding HTTP endpoint
# Try both HTTP and WebSocket ports
if command -v websocat >/dev/null 2>&1; then
  # WebSocket client available
  LOCAL_BASE_BLOCK=$(echo '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' | websocat -n1 ws://localhost:8546 2>/dev/null | jq -r '.result' 2>/dev/null | xargs printf "%d" 2>/dev/null || echo "N/A")
else
  # Try HTTP on same port (common setup)
  LOCAL_BASE_BLOCK=$(curl -s -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' http://localhost:8545 2>/dev/null | jq -r '.result' 2>/dev/null | xargs printf "%d" 2>/dev/null || echo "N/A")

  # If that fails, try the WebSocket port but with HTTP
  if [ "$LOCAL_BASE_BLOCK" = "N/A" ]; then
    LOCAL_BASE_BLOCK=$(curl -s -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' http://localhost:8546 2>/dev/null | jq -r '.result' 2>/dev/null | xargs printf "%d" 2>/dev/null || echo "N/A")
  fi
fi

# Calculate difference between public and local nodes
ETH_DIFF="N/A"
BASE_DIFF="N/A"
ETH_DIFF_FORMATTED=""
BASE_DIFF_FORMATTED=""

if [ "$LOCAL_ETH_BLOCK" != "N/A" ]; then
  ETH_DIFF=$((ETH_BLOCK - LOCAL_ETH_BLOCK))
  # Format with appropriate sign and color
  if [ $ETH_DIFF -gt 0 ]; then
    ETH_DIFF_FORMATTED="(behind: +$ETH_DIFF)"
  elif [ $ETH_DIFF -lt 0 ]; then
    ETH_DIFF_FORMATTED="(ahead: $ETH_DIFF)"
  else
    ETH_DIFF_FORMATTED="(in sync)"
  fi
  # Format the local block number with thousands separators
  LOCAL_ETH_BLOCK=$(printf "%'d" $LOCAL_ETH_BLOCK)
fi

if [ "$LOCAL_BASE_BLOCK" != "N/A" ]; then
  BASE_DIFF=$((BASE_BLOCK - LOCAL_BASE_BLOCK))
  # Format with appropriate sign and color
  if [ $BASE_DIFF -gt 0 ]; then
    BASE_DIFF_FORMATTED="(behind: +$BASE_DIFF)"
  elif [ $BASE_DIFF -lt 0 ]; then
    BASE_DIFF_FORMATTED="(ahead: $BASE_DIFF)"
  else
    BASE_DIFF_FORMATTED="(in sync)"
  fi
  # Format the local block number with thousands separators
  LOCAL_BASE_BLOCK=$(printf "%'d" $LOCAL_BASE_BLOCK)
fi

# Output formatted for the dashboard
cat << EOF
Ethereum (Public) : $(printf "%'d" $ETH_BLOCK)
Ethereum (Local)  : $LOCAL_ETH_BLOCK $ETH_DIFF_FORMATTED
Base (Public)     : $(printf "%'d" $BASE_BLOCK)
Base (Local)      : $LOCAL_BASE_BLOCK $BASE_DIFF_FORMATTED
EOF