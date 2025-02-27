// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.27;

import "forge-std/Test.sol";
import "../src/UniswapQuery.sol";

contract MockERC20 is IERC20 {
    string private _name;
    string private _symbol;
    uint8 private _decimals;

    constructor(string memory name_, string memory symbol_, uint8 decimals_) {
        _name = name_;
        _symbol = symbol_;
        _decimals = decimals_;
    }

    function name() external view override returns (string memory) {
        return _name;
    }

    function symbol() external view override returns (string memory) {
        return _symbol;
    }

    function decimals() external view override returns (uint8) {
        return _decimals;
    }

    function totalSupply() external pure override returns (uint256) {
        return 0;
    }

    function balanceOf(address) external pure override returns (uint256) {
        return 0;
    }

    function allowance(address, address) external pure override returns (uint256) {
        return 0;
    }

    function approve(address, uint256) external pure override returns (bool) {
        return true;
    }

    function transfer(address, uint256) external pure override returns (bool) {
        return true;
    }

    function transferFrom(address, address, uint256) external pure override returns (bool) {
        return true;
    }
}

contract MockUniswapV2Pair is IUniswapV2Pair {
    address private immutable _token0;
    address private immutable _token1;

    constructor(address token0_, address token1_) {
        _token0 = token0_;
        _token1 = token1_;
    }

    function token0() external view override returns (address) {
        return _token0;
    }

    function token1() external view override returns (address) {
        return _token1;
    }

    function getReserves() external view override returns (uint112, uint112, uint32) {
        return (1000, 1000, uint32(block.timestamp));
    }
}

contract MockUniswapV2Factory is UniswapV2Factory {
    function addPair(address pair) external {
        allPairs.push(pair);
    }

    function allPairsLength() external view override returns (uint256) {
        return allPairs.length;
    }
}

contract UniswapQueryTest is Test {
    UniswapQuery public query;
    MockUniswapV2Factory public factory;
    MockERC20 public token0;
    MockERC20 public token1;
    MockUniswapV2Pair public pair;

    function setUp() public {
        query = new UniswapQuery();
        factory = new MockUniswapV2Factory();

        // Create mock tokens
        token0 = new MockERC20("Token0", "TKN0", 18);
        token1 = new MockERC20("Token1", "TKN1", 6);

        // Create mock pair
        pair = new MockUniswapV2Pair(address(token0), address(token1));

        // Add pair to factory
        factory.addPair(address(pair));
    }

    function testGetPairsByIndexRange() public {
        UniswapQuery.PairInfo[] memory pairs = query.getPairsByIndexRange(factory, 0, 1);

        assertEq(pairs.length, 1, "Should return one pair");
        assertEq(pairs[0].pairAddress, address(pair), "Wrong pair address");
        assertEq(pairs[0].token0.tokenAddress, address(token0), "Wrong token0 address");
        assertEq(pairs[0].token1.tokenAddress, address(token1), "Wrong token1 address");
        assertEq(pairs[0].token0.name, "Token0", "Wrong token0 name");
        assertEq(pairs[0].token1.name, "Token1", "Wrong token1 name");
        assertEq(pairs[0].token0.symbol, "TKN0", "Wrong token0 symbol");
        assertEq(pairs[0].token1.symbol, "TKN1", "Wrong token1 symbol");
        assertEq(pairs[0].token0.decimals, 18, "Wrong token0 decimals");
        assertEq(pairs[0].token1.decimals, 6, "Wrong token1 decimals");
    }

    function testGetReservesByPairs() public {
        IUniswapV2Pair[] memory pairs = new IUniswapV2Pair[](1);
        pairs[0] = pair;

        uint256[3][] memory reserves = query.getReservesByPairs(pairs);

        assertEq(reserves.length, 1, "Should return reserves for one pair");
        assertEq(reserves[0][0], 1000, "Wrong reserve0");
        assertEq(reserves[0][1], 1000, "Wrong reserve1");
    }

    function testAllPairsLength() public {
        uint256 length = query.allPairsLength(factory);
        assertEq(length, 1, "Wrong pairs length");
    }
}
