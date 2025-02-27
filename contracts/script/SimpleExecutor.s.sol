// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {SimpleExecutor} from "../src/SimpleExecutor.sol";

contract SimpleExecutorScript is Script {
    SimpleExecutor public simpleExecutor;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        simpleExecutor = new SimpleExecutor();

        vm.stopBroadcast();
    }
}
