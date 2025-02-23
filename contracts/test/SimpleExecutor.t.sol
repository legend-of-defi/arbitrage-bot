// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {SimpleExecutor} from "../src/SimpleExecutor.sol";

// Helper contract for testing
contract MockTarget {
    uint256 public value;
    bool public shouldRevert;

    function setValue(uint256 _value) external {
        require(!shouldRevert, "MockTarget: revert requested");
        value = _value;
    }

    function setRevertStatus(bool _shouldRevert) external {
        shouldRevert = _shouldRevert;
    }
}

contract SimpleExecutorTest is Test {
    SimpleExecutor public simpleExecutor;
    MockTarget public mockTarget1;
    MockTarget public mockTarget2;

    function setUp() public {
        simpleExecutor = new SimpleExecutor();
        mockTarget1 = new MockTarget();
        mockTarget2 = new MockTarget();
    }

    function test_SingleSuccessfulCall() public {
        address[] memory targets = new address[](1);
        targets[0] = address(mockTarget1);

        bytes[] memory payloads = new bytes[](1);
        payloads[0] = abi.encodeWithSignature("setValue(uint256)", 42);

        simpleExecutor.run(targets, payloads);
        assertEq(mockTarget1.value(), 42);
    }

    function test_MultipleSuccessfulCalls() public {
        address[] memory targets = new address[](2);
        targets[0] = address(mockTarget1);
        targets[1] = address(mockTarget2);

        bytes[] memory payloads = new bytes[](2);
        payloads[0] = abi.encodeWithSignature("setValue(uint256)", 42);
        payloads[1] = abi.encodeWithSignature("setValue(uint256)", 24);

        simpleExecutor.run(targets, payloads);
        assertEq(mockTarget1.value(), 42);
        assertEq(mockTarget2.value(), 24);
    }

    function test_RevertOnFailedCall() public {
        address[] memory targets = new address[](2);
        targets[0] = address(mockTarget1);
        targets[1] = address(mockTarget2);

        bytes[] memory payloads = new bytes[](2);
        payloads[0] = abi.encodeWithSignature("setValue(uint256)", 42);
        payloads[1] = abi.encodeWithSignature("setValue(uint256)", 24);

        mockTarget2.setRevertStatus(true);

        // Don't check specific message, just verify it reverts
        vm.expectRevert();
        simpleExecutor.run(targets, payloads);

        assertEq(mockTarget1.value(), 0);
    }

    function testFuzz_SingleCall(uint256 value) public {
        address[] memory targets = new address[](1);
        targets[0] = address(mockTarget1);

        bytes[] memory payloads = new bytes[](1);
        payloads[0] = abi.encodeWithSignature("setValue(uint256)", value);

        simpleExecutor.run(targets, payloads);
        assertEq(mockTarget1.value(), value);
    }

    function test_EmptyArrays() public {
        address[] memory targets = new address[](0);
        bytes[] memory payloads = new bytes[](0);

        // Should execute successfully with no effects
        simpleExecutor.run(targets, payloads);
    }

    function test_RevertOnArrayLengthMismatch() public {
        address[] memory targets = new address[](2);
        targets[0] = address(mockTarget1);
        targets[1] = address(mockTarget2);

        bytes[] memory payloads = new bytes[](1);
        payloads[0] = abi.encodeWithSignature("setValue(uint256)", 42);

        vm.expectRevert();
        simpleExecutor.run(targets, payloads);
    }
}
