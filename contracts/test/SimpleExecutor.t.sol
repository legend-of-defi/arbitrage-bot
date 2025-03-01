// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/Test.sol";
import {SimpleExecutor, IUniV2Pair} from "../src/SimpleExecutor.sol";
import {StdChains} from "forge-std/StdChains.sol";

interface IERC20 {
    function balanceOf(address account) external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
}

// Contract to test `call` functionality
contract TargetContract {
    uint256 public storedValue;

    function setValue(uint256 _value) external {
        storedValue = _value;
    }

    function getValue() external view returns (uint256) {
        return storedValue;
    }
}

contract TargetContractWithRevert {
    function revertingFunction() external pure {
        revert("Revert in TargetContract");
    }
}

contract MockFailingERC20 {
    function balanceOf(address) external pure returns (uint256) {
        return 1000 * 1e18; // Return some fake balance
    }

    function transfer(address, uint256) external pure returns (bool) {
        return false; // Always fail the transfer
    }
}

contract SimpleExecutorTest is Test {
    SimpleExecutor public executor;
    TargetContract public targetContract;

    address public owner;
    address public targetAddress;
    address public nonOwner;

    // Mainnet addresses
    address constant WETH = 0x4200000000000000000000000000000000000006;
    address constant USDC = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913;
    address constant UNIV2_USDC_WETH = 0x88A43bbDF9D098eEC7bCEda4e2494615dfD9bB9C; // Uniswap V2
    address constant STANDARD_UNIV2_PAIR = 0xaEeB835f3Aa21d19ea5E33772DaA9E64f1b6982F; // Standard Uniswap V2 pair (suggested)

    function setUp() public {
        // Set the owner address to the current contract (the contract itself)
        owner = address(this);

        // Deploy a new SimpleExecutor contract (the contract being tested)
        executor = new SimpleExecutor();

        // Deploy a new TargetContract (a contract that will be called by the executor)
        targetContract = new TargetContract();

        // Store the address of the deployed target contract for use in tests
        targetAddress = address(targetContract);

        // Define a non-owner address (an address that is not the owner of the Executor contract)
        // This address will be used to test access restrictions for non-owners
        nonOwner = address(0x1234567890123456789012345678901234567890);

        // Fork mainnet using StdChains RPC URL
        vm.createSelectFork(getChain("base").rpcUrl);
    }

    function test_SuccessfulArbitrage() public {
        // Set fixed reserves using original values with a slight adjustment to the second pair
        uint112 fixedWETHReserveUni = 1010.78 ether;
        uint112 fixedUSDCReserveUni = 2_513_107e6;
        uint112 fixedWETHReserveStandard = 0.86 ether;
        uint112 fixedUSDCReserveStandard = 2_314e6; // Increased from 2_014e6 to create a better arbitrage opportunity

        // Override reserves in the actual pair contracts
        mockPairReserves(UNIV2_USDC_WETH, fixedWETHReserveUni, fixedUSDCReserveUni);
        mockPairReserves(STANDARD_UNIV2_PAIR, fixedWETHReserveStandard, fixedUSDCReserveStandard);

        // Give tokens to the executor and pairs
        deal(USDC, address(executor), 100_000e6);
        deal(WETH, address(executor), 10_000 ether);

        deal(WETH, UNIV2_USDC_WETH, fixedWETHReserveUni);
        deal(USDC, UNIV2_USDC_WETH, fixedUSDCReserveUni);

        deal(WETH, STANDARD_UNIV2_PAIR, fixedWETHReserveStandard);
        deal(USDC, STANDARD_UNIV2_PAIR, fixedUSDCReserveStandard);

        // Start with USDC, use a small amount to minimize price impact
        uint256 amountIn = 100e6; // 100 USDC (reduced from 50,000)

        // Calculate expected outputs
        // For Uniswap: USDC -> WETH
        uint256 amountOutUni = getAmountOut(amountIn, fixedUSDCReserveUni, fixedWETHReserveUni);
        // For Standard Uniswap V2 pair: WETH -> USDC
        uint256 amountOutStandard = getAmountOut(amountOutUni, fixedWETHReserveStandard, fixedUSDCReserveStandard);

        // Create pairs array for the executor
        SimpleExecutor.Pair[] memory pairs = new SimpleExecutor.Pair[](2);

        // First swap: USDC -> WETH on Uniswap
        pairs[0] = SimpleExecutor.Pair({
            contractAddress: UNIV2_USDC_WETH,
            amountOut: amountOutUni,
            isToken0: true // WETH is token0
        });

        // Second swap: WETH -> USDC on Standard Uniswap V2 pair
        pairs[1] = SimpleExecutor.Pair({
            contractAddress: STANDARD_UNIV2_PAIR,
            amountOut: amountOutStandard,
            isToken0: false // USDC is token1
        });

        // Execute the arbitrage with profit check enabled
        executor.run(
            USDC,
            amountIn,
            0.01e6, // Minimum profit of 0.01 USDC
            pairs,
            false // skipProfitCheck = false
        );
    }

    function testRevert_IfInsufficientBalance() public {
        // Set reserves
        uint112 reserve0 = 100_000e6; // 100,000 USDC
        uint112 reserve1 = 0.5 ether; // 0.5 WETH
        mockPairReserves(UNIV2_USDC_WETH, reserve0, reserve1);

        // Don't give executor any USDC - balance will be 0 by default

        SimpleExecutor.Pair[] memory pairs = new SimpleExecutor.Pair[](1);
        pairs[0] = SimpleExecutor.Pair({contractAddress: UNIV2_USDC_WETH, amountOut: 4935790171985306, isToken0: true});

        vm.expectRevert("ERC20: transfer amount exceeds balance");
        executor.run(USDC, 1000e6, 27e6, pairs, false);
    }

    function test_WithdrawAsOwner() public {
        vm.deal(address(executor), 1 ether);
        uint256 initialBalance = address(this).balance;
        executor.withdraw();
        assertEq(address(this).balance, initialBalance + 1 ether);
        assertEq(address(executor).balance, 0);
    }

    function testRevert_IfWithdrawAsNonOwner() public {
        vm.deal(address(executor), 1 ether);
        vm.prank(address(0xdead));
        vm.expectRevert(SimpleExecutor.NotOwner.selector);
        executor.withdraw();
    }

    function testRevert_IfProfitMarginNotMet() public {
        // Set reserves where arbitrage will result in a loss
        uint112 fixedUSDCReserveUni = 100_000e6;
        uint112 fixedWETHReserveUni = 0.5 ether;
        uint112 fixedUSDCReserveStandard = 80_000e6;
        uint112 fixedWETHReserveStandard = 0.45 ether;

        // Make sure reserves are set correctly (WETH is token0, USDC is token1)
        mockPairReserves(UNIV2_USDC_WETH, fixedWETHReserveUni, fixedUSDCReserveUni);
        mockPairReserves(STANDARD_UNIV2_PAIR, fixedWETHReserveStandard, fixedUSDCReserveStandard);

        // Give executor initial USDC
        deal(USDC, address(executor), 1000e6);

        // Give tokens to the pairs (very important for the swaps to work)
        deal(WETH, UNIV2_USDC_WETH, fixedWETHReserveUni * 10); // Give 10x the reserves to ensure sufficient liquidity
        deal(USDC, UNIV2_USDC_WETH, fixedUSDCReserveUni * 10);
        deal(WETH, STANDARD_UNIV2_PAIR, fixedWETHReserveStandard * 10);
        deal(USDC, STANDARD_UNIV2_PAIR, fixedUSDCReserveStandard * 10);

        // Calculate expected amounts
        uint256 expectedWETHOutUni = getAmountOut(1000e6, fixedUSDCReserveUni, fixedWETHReserveUni);
        uint256 expectedUSDCOutStandard =
            getAmountOut(expectedWETHOutUni, fixedWETHReserveStandard, fixedUSDCReserveStandard);
        SimpleExecutor.Pair[] memory pairs = new SimpleExecutor.Pair[](2);

        pairs[0] =
            SimpleExecutor.Pair({contractAddress: UNIV2_USDC_WETH, amountOut: expectedWETHOutUni, isToken0: true});

        pairs[1] = SimpleExecutor.Pair({
            contractAddress: STANDARD_UNIV2_PAIR,
            amountOut: expectedUSDCOutStandard,
            isToken0: false
        });

        int256 actualProfit = int256(expectedUSDCOutStandard) - int256(1000e6);
        assertEq(actualProfit, -134621970);

        // Expect revert because we'll get back less USDC than we put in
        vm.expectRevert(
            abi.encodeWithSelector(
                SimpleExecutor.ProfitTargetNotMet.selector,
                27e6, // Require 27 USDC profit
                -134621970 // Get -134.621970 USDC
            )
        );
        executor.run(USDC, 1000e6, 27e6, pairs, false);
    }

    function test_SkipProfitCheck() public {
        // Set reserves where arbitrage would normally result in a loss
        uint112 fixedUSDCReserveUni = 100_000e6;
        uint112 fixedWETHReserveUni = 0.5 ether;
        uint112 fixedUSDCReserveStandard = 80_000e6;
        uint112 fixedWETHReserveStandard = 0.45 ether;

        mockPairReserves(UNIV2_USDC_WETH, fixedWETHReserveUni, fixedUSDCReserveUni);
        mockPairReserves(STANDARD_UNIV2_PAIR, fixedWETHReserveStandard, fixedUSDCReserveStandard);

        deal(USDC, address(executor), 1000e6);

        deal(WETH, UNIV2_USDC_WETH, fixedWETHReserveUni);
        deal(USDC, UNIV2_USDC_WETH, fixedUSDCReserveUni);
        deal(WETH, STANDARD_UNIV2_PAIR, fixedWETHReserveStandard);
        deal(USDC, STANDARD_UNIV2_PAIR, fixedUSDCReserveStandard);

        uint256 expectedWETHOutUni = getAmountOut(1000e6, fixedUSDCReserveUni, fixedWETHReserveUni);
        uint256 expectedUSDCOutStandard =
            getAmountOut(expectedWETHOutUni, fixedWETHReserveStandard, fixedUSDCReserveStandard);

        SimpleExecutor.Pair[] memory pairs = new SimpleExecutor.Pair[](2);

        pairs[0] =
            SimpleExecutor.Pair({contractAddress: UNIV2_USDC_WETH, amountOut: expectedWETHOutUni, isToken0: true});

        pairs[1] = SimpleExecutor.Pair({
            contractAddress: STANDARD_UNIV2_PAIR,
            amountOut: expectedUSDCOutStandard,
            isToken0: false
        });

        // This would normally revert due to insufficient profit, but should pass with skipProfitCheck = true
        executor.run(USDC, 1000e6, 27e6, pairs, true);

        // Verify the swap happened despite the loss
        uint256 finalBalance = IERC20(USDC).balanceOf(address(executor));
        assertLt(finalBalance, 1000e6); // Balance should be less than initial amount
    }

    // ---------------------- Call Method Tests Begin ----------------------

    // Test: Call contract as the owner (successful call)
    function testCallAsOwner() public {
        // Prepare the data to call the 'setValue' function of the target contract
        bytes memory data = abi.encodeWithSignature("setValue(uint256)", 42);

        // Convert targetAddress to a payable address
        address payable targetAddressPayable = payable(targetAddress);

        // Call the contract using the Executor's callContract method
        (bytes memory result) = executor.callContract(targetAddressPayable, 0, data);

        // Assert that the result is empty (TargetContract doesn't return anything)
        assertEq(result.length, 0);

        // Assert that the target contract's stored value has been updated correctly
        assertEq(targetContract.getValue(), 42);
    }

    // Test: Call contract as a non-owner (should revert)
    function testCallAsNonOwner() public {
        // Simulate the call coming from a non-owner
        vm.prank(nonOwner);

        // Prepare the data to call the 'setValue' function of the target contract
        bytes memory data = abi.encodeWithSignature("setValue(uint256)", 42);

        // Convert targetAddress to a payable address
        address payable targetAddressPayable = payable(targetAddress);

        // Expect revert due to 'NotOwner' error in the Executor contract
        vm.expectRevert(SimpleExecutor.NotOwner.selector);

        // Attempt the call, expecting it to fail
        executor.callContract(targetAddressPayable, 0, data);
    }

    // Test: Call contract with an invalid address (should revert)
    function testCallToInvalidAddress() public {
        // Set an invalid address (0 address)
        address payable invalidAddress = payable(address(0));

        // Prepare the data to call the 'setValue' function of the target contract
        bytes memory data = abi.encodeWithSignature("setValue(uint256)", 42);

        // Expect revert due to 'InvalidAddress' error in the Executor contract
        vm.expectRevert(SimpleExecutor.InvalidAddress.selector);

        // Attempt the call, expecting it to fail
        executor.callContract(invalidAddress, 0, data);
    }

    // Test: Call contract with a revert in the target contract (should revert)
    function testCallWithRevertInTarget() public {
        // Deploy a target contract that has a function that always reverts
        TargetContractWithRevert targetContractWithRevert = new TargetContractWithRevert();

        // Convert target address to a payable address
        address payable targetAddressPayable = payable(address(targetContractWithRevert));

        // Prepare the data to call the 'revertingFunction' in the target contract
        bytes memory data = abi.encodeWithSignature("revertingFunction()");

        // Expect revert due to 'CallFailed' error in the Executor contract
        vm.expectRevert(SimpleExecutor.CallFailed.selector);

        // Attempt the call, expecting it to fail
        executor.callContract(targetAddressPayable, 0, data);
    }

    // ---------------------- Call Method Tests End ----------------------

    // ---------------------- Withdraw ERC20 Tests Begin ----------------------

    // Test: Withdraw ERC20 tokens as the owner (successful withdrawal)
    function testWithdrawERC20AsOwner() public {
        // Give the Executor contract 1000 USDC tokens
        deal(USDC, address(executor), 1000e6);

        // Record the initial balance of the owner
        uint256 initialOwnerBalance = IERC20(USDC).balanceOf(address(this));

        // Call the withdraw function to transfer 1000 USDC to the owner
        executor.withdrawERC20(USDC, owner, 1000e6);

        // Record the final balance of the owner after the withdrawal
        uint256 finalOwnerBalance = IERC20(USDC).balanceOf(address(this));

        // Assert that the owner has received the correct amount
        assertEq(finalOwnerBalance, initialOwnerBalance + 1000e6, "Owner should receive the withdrawn amount");

        // Assert that the Executor contract's balance is now 0
        assertEq(
            IERC20(USDC).balanceOf(address(executor)), 0, "Executor contract should have 0 balance after withdrawal"
        );
    }

    // Test: Revert ERC20 withdrawal by non-owner (should fail)
    function testRevert_WithdrawERC20AsNonOwner() public {
        // Give the Executor contract 1000 USDC tokens
        deal(USDC, address(executor), 1000e6);

        // Simulate the call coming from a non-owner
        vm.prank(nonOwner);

        // Expect revert due to 'NotOwner' error in the Executor contract
        vm.expectRevert(SimpleExecutor.NotOwner.selector);

        // Attempt the withdrawal, which should fail for non-owner
        executor.withdrawERC20(USDC, nonOwner, 1000e6);
    }

    // Test: Revert ERC20 withdrawal when the contract has no balance (should fail)
    function testRevert_WithdrawERC20WhenEmpty() public {
        // Expect revert due to 'NoBalanceToWithdraw' error (contract has no balance to withdraw)
        vm.expectRevert(SimpleExecutor.NoBalanceToWithdraw.selector);

        // Attempt the withdrawal, which should fail as the contract has no USDC balance
        executor.withdrawERC20(address(USDC), address(this), 1e9);
    }

    // Test: Revert ERC20 withdrawal due to failed transfer (should fail)
    function testRevert_WithdrawERC20TransferFailed() public {
        // Deploy a mock ERC20 token that always fails the transfer
        MockFailingERC20 mockToken = new MockFailingERC20();

        // Expect revert due to 'ERC20Failed' error in the Executor contract
        vm.expectRevert(SimpleExecutor.ERC20Failed.selector);

        // Attempt the withdrawal, which should fail due to the transfer failure
        executor.withdrawERC20(address(mockToken), address(this), 1e9);
    }

    // ---------------------- Withdraw ERC20 Tests End ----------------------

    // Allow this contract to receive ETH
    receive() external payable {}

    // Helper function to mock pair reserves
    // We are forking mainnet, so the balances are undefined and for tests we need to set them.
    function mockPairReserves(address pair, uint112 reserve0, uint112 reserve1) internal {
        uint32 blockTimestampLast = uint32(block.timestamp);
        bytes32 value;
        assembly {
            // Pack reserve0 (112 bits) | reserve1 (112 bits) | blockTimestampLast (32 bits)
            value := or(or(reserve0, shl(112, reserve1)), shl(224, blockTimestampLast))
        }
        vm.store(pair, bytes32(uint256(8)), value); // Slot 8 is where UniswapV2Pair stores reserves
    }

    // Helper function to calculate the expected output amount
    // In production this will be done off-chain and passed to the executor as parameter.
    // The executor will then compare the expected reserves to the actual reserves.
    // If they don't match, it will revert.
    function getAmountOut(uint256 amountIn, uint256 reserveIn, uint256 reserveOut)
        internal
        pure
        returns (uint256 amountOut)
    {
        uint256 amountInWithFee = amountIn * 997;
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = reserveIn * 1000 + amountInWithFee;
        amountOut = numerator / denominator;
    }
}
