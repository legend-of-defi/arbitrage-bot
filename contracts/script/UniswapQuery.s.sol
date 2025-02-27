// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.27;

import "forge-std/Script.sol";
import "../src/UniswapQuery.sol";

contract UniswapQueryScript is Script {
    function run() external {
        string memory privateKey = vm.envString("PRIVATE_KEY");
        uint256 deployerPrivateKey = vm.parseUint(string.concat("0x", privateKey));

        vm.startBroadcast(deployerPrivateKey);

        UniswapQuery query = new UniswapQuery();

        vm.stopBroadcast();

        console.log("UniswapQuery deployed to:", address(query));
    }
}
