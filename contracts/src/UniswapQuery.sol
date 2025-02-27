// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.27;

pragma experimental ABIEncoderV2;

interface IUniswapV2Pair {
    function token0() external view returns (address);
    function token1() external view returns (address);
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
}

interface IERC20 {
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event Transfer(address indexed from, address indexed to, uint256 value);

    function name() external view returns (string memory);
    function symbol() external view returns (string memory);
    function decimals() external view returns (uint8);
    function totalSupply() external view returns (uint256);
    function balanceOf(address owner) external view returns (uint256);
    function allowance(address owner, address spender) external view returns (uint256);

    function approve(address spender, uint256 value) external returns (bool);
    function transfer(address to, uint256 value) external returns (bool);
    function transferFrom(address from, address to, uint256 value) external returns (bool);
}

abstract contract UniswapV2Factory {
    mapping(address => mapping(address => address)) public getPair;
    address[] public allPairs;

    function allPairsLength() external view virtual returns (uint256);
}

// In order to quickly load up data from Uniswap-like market, this contract allows easy iteration with a single eth_call
contract UniswapQuery {
    struct Token {
        address tokenAddress;
        string name;
        string symbol;
        uint8 decimals;
    }

    struct PairInfo {
        Token token0;
        Token token1;
        address pairAddress;
    }

    function allPairsLength(UniswapV2Factory _uniswapFactory) external view returns (uint256) {
        return _uniswapFactory.allPairsLength();
    }

    function getTokenInfo(address tokenAddress) internal view returns (Token memory) {
        IERC20 token = IERC20(tokenAddress);
        return
            Token({tokenAddress: tokenAddress, name: token.name(), symbol: token.symbol(), decimals: token.decimals()});
    }

    function getReservesByPairs(IUniswapV2Pair[] calldata _pairs) external view returns (uint256[3][] memory) {
        uint256[3][] memory result = new uint256[3][](_pairs.length);
        for (uint256 i = 0; i < _pairs.length; i++) {
            (result[i][0], result[i][1], result[i][2]) = _pairs[i].getReserves();
        }
        return result;
    }

    function getPairsByIndexRange(UniswapV2Factory _uniswapFactory, uint256 _start, uint256 _stop)
        external
        view
        returns (PairInfo[] memory)
    {
        uint256 _allPairsLength = _uniswapFactory.allPairsLength();
        if (_stop > _allPairsLength) {
            _stop = _allPairsLength;
        }
        require(_stop >= _start, "start cannot be higher than stop");

        uint256 _qty = _stop - _start;
        PairInfo[] memory result = new PairInfo[](_qty);

        for (uint256 i = 0; i < _qty; i++) {
            address pairAddress = _uniswapFactory.allPairs(_start + i);
            IUniswapV2Pair pair = IUniswapV2Pair(pairAddress);

            // Get token addresses
            address token0Address = pair.token0();
            address token1Address = pair.token1();

            // Get token information
            Token memory token0Info = getTokenInfo(token0Address);
            Token memory token1Info = getTokenInfo(token1Address);

            // Store all information
            result[i] = PairInfo({token0: token0Info, token1: token1Info, pairAddress: pairAddress});
        }

        return result;
    }
}
