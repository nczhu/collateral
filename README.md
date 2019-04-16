# Simple Collateral

A implementation of tokenized debt. A token owner can collateralize a token for a loan, and pay back with interest. Creditors can seize the collateral on defaulted loans.

Modules: 
`Debt`: handles creation of loan requests, paybacks, seizing of collateral.
`ERC721`: adapted from this (sample)[https://github.com/parity-samples/substrate-erc721/tree/master/substrate-erc721] to also be able to collateralize and uncollateralize tokens for a `reason`.


### Run 
```
./build.sh
cargo build
./release...
```

### Run Tests
`cargo test -p node-template-runtime -- --nocapture`