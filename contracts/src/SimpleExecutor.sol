// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

interface IUniV2Pair {
    function swap(uint256, uint256, address, bytes calldata) external;
}

interface IERC20 {
    function transfer(address, uint256) external returns (bool);
    function balanceOf(address) external view returns (uint256);
}

/// @title SimpleExecutor
/// @notice Executes arbitrage trades across Uniswap V2 pairs
/// @dev Requires approval for token0 before execution
contract SimpleExecutor {
    address public immutable owner;

    error NotOwner();
    error WithdrawalFailed();
    error ProfitTargetNotMet(uint256 minimumProfit, int256 actualProfit);
    error InvalidPairCount();
    error CallFailed();
    error InvalidAddress();
    error ERC20Failed();
    error NoBalanceToWithdraw();

    event FailureInfo(uint112 indexed index, string reason);

    // Argument for each pair
    struct Pair {
        address contractAddress; // 20 bytes
        uint256 amountOut; // amount0Out or amount1Out to be passed to swap
        bool isToken0; // 1 byte
            // Total: 33 bytes packed into a single 32-byte slot
    }

    constructor() {
        owner = msg.sender;
    }

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    /// @notice Withdraws all ETH from the contract
    /// @dev Uses low-level call intentionally to avoid gas limitations of transfer()
    /// @custom:slither-disable-next-line low-level-calls
    function withdraw() external onlyOwner {
        (bool success,) = owner.call{value: address(this).balance}("");
        if (!success) revert WithdrawalFailed();
    }

    /// @notice Withdraws ERC20 tokens from the contract
    /// @param token Address of the ERC20 token
    /// @param recipient Address to send the tokens to
    /// @param amount Amount of tokens to withdraw
    function withdrawERC20(address token, address recipient, uint256 amount) external onlyOwner {
        uint256 contractBalance = IERC20(token).balanceOf(address(this));

        if (contractBalance < amount) revert NoBalanceToWithdraw();

        bool success = IERC20(token).transfer(recipient, amount);
        if (!success) revert ERC20Failed();
    }

    /// @notice Executes an arbitrary low-level call to another contract or address
    /// @dev This function allows the owner to call any external contract with custom calldata
    /// @param _to The address of the target contract or recipient
    /// @param _value The amount of ETH (in wei) to send with the call
    /// @param _data Encoded function call data to be executed
    /// @return result The raw return data from the executed call
    function callContract(address payable _to, uint256 _value, bytes calldata _data)
        external
        payable
        onlyOwner
        returns (bytes memory result)
    {
        // Ensure that the target address is not the zero address to prevent unintended behavior.
        if (_to == address(0)) revert InvalidAddress();

        // Perform a low-level call to `_to` with `_value` of ETH and `_data` as calldata.
        // `_success` indicates whether the call was successful.
        // `_result` contains any return data from the called contract.
        (bool success, bytes memory returnData) = _to.call{value: _value}(_data);

        // Revert the transaction if the call fails to prevent further execution.
        if (!success) revert CallFailed();

        // Return the raw data received from the called contract.
        return returnData;
    }

    /// @notice Executes a series of swaps for arbitrage
    /// @param token0Address Address of the token to trade
    /// @param token0AmountIn Initial amount of token0 to trade
    /// @param minimumProfitInToken0 Minimum acceptable profit in token0
    /// @param pairs Array of pairs to trade through
    /// @param skipProfitCheck Whether to skip profit check
    /// @dev Reverts if profit target not met
    function run(
        address token0Address,
        uint256 token0AmountIn,
        uint256 minimumProfitInToken0,
        Pair[] calldata pairs,
        bool skipProfitCheck
    ) external payable onlyOwner {
        uint256 pairsLength = pairs.length;
        if (pairsLength == 0 || pairsLength > 5) revert InvalidPairCount();

        address self = address(this);
        IERC20 token0Contract = IERC20(token0Address);
        uint256 token0BalanceBefore = token0Contract.balanceOf(self);

        token0Contract.transfer(pairs[0].contractAddress, token0AmountIn);

        // Optimize loop
        unchecked {
            // Add unchecked for gas savings since we know length is small
            for (uint256 i; i < pairsLength;) {
                Pair calldata pair = pairs[i];
                address recipient = i == pairsLength - 1 ? self : pairs[i + 1].contractAddress;

                // Avoid duplicate ternary operations
                uint256 amount0Out = pair.isToken0 ? pair.amountOut : 0;
                uint256 amount1Out = pair.isToken0 ? 0 : pair.amountOut;

                IUniV2Pair(pair.contractAddress).swap(amount0Out, amount1Out, recipient, "");
                ++i;
            }
        }

        // Move profit check into unchecked block
        unchecked {
            if (!skipProfitCheck) {
                int256 profit = int256(token0Contract.balanceOf(self) - token0BalanceBefore);
                if (profit < int256(minimumProfitInToken0)) {
                    revert ProfitTargetNotMet(minimumProfitInToken0, profit);
                }
            }
        }
    }
}
