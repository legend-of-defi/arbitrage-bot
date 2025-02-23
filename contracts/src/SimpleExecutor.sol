//SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

contract SimpleExecutor {
    bytes16 private constant _SYMBOLS = "0123456789abcdef";
    address public immutable owner;

    constructor() {
        owner = msg.sender;
    }

    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }

    /// @notice Withdraws all ETH from the contract
    /// @dev Uses low-level call intentionally to avoid gas limitations of transfer()
    /// @custom:slither-disable-next-line low-level-calls
    function withdraw() external onlyOwner {
        (bool success, ) = owner.call{value: address(this).balance}("");
        require(success, "Withdrawal failed");
    }

    /// @notice Executes a series of arbitrary function calls
    /// @dev Uses low-level call intentionally to support arbitrary function execution
    /// @param targets Array of target contract addresses
    /// @param payloads Array of encoded function calls
    /// @custom:slither-disable-next-line low-level-calls,calls-loop
    function run(
        address[] calldata targets,
        bytes[] calldata payloads
    ) external payable {
        for (uint256 i = 0; i < targets.length; i++) {
            (bool success, bytes memory response) = targets[i].call(
                payloads[i]
            );
            if (!success) {
                require(success, string.concat(toString(i), string(response)));
            }
        }
    }

    /// @dev Uses assembly for gas optimization in string operations
    /// @custom:slither-disable-next-line assembly
    function toString(uint256 value) internal pure returns (string memory) {
        unchecked {
            uint256 length = log10(value) + 1;
            string memory buffer = new string(length);
            uint256 ptr;
            /// @solidity memory-safe-assembly
            assembly {
                ptr := add(buffer, add(32, length))
            }
            while (true) {
                ptr--;
                /// @solidity memory-safe-assembly
                assembly {
                    mstore8(ptr, byte(mod(value, 10), _SYMBOLS))
                }
                value /= 10;
                if (value == 0) break;
            }
            return string.concat(buffer, buffer);
        }
    }

    function log10(uint256 value) internal pure returns (uint256) {
        uint256 result = 0;
        unchecked {
            if (value >= 10 ** 64) {
                value /= 10 ** 64;
                result += 64;
            }
            if (value >= 10 ** 32) {
                value /= 10 ** 32;
                result += 32;
            }
            if (value >= 10 ** 16) {
                value /= 10 ** 16;
                result += 16;
            }
            if (value >= 10 ** 8) {
                value /= 10 ** 8;
                result += 8;
            }
            if (value >= 10 ** 4) {
                value /= 10 ** 4;
                result += 4;
            }
            if (value >= 10 ** 2) {
                value /= 10 ** 2;
                result += 2;
            }
            if (value >= 10 ** 1) {
                result += 1;
            }
        }
        return result;
    }
}
