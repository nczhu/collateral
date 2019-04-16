# Simple Collateral

A simple implementation of tokenized debt, with simple-interest. A token owner can collateralize a non-fungible token for a loan, and pay it back with interest. Creditors can seize the collateral upon loan default.

### Modules: 
* `Debt`: handles creation of loan requests, paybacks, seizing of collateral.
* `ERC721`: adapted from this (sample)[https://github.com/parity-samples/substrate-erc721/tree/master/substrate-erc721] to also be able to collateralize and uncollateralize tokens for a `reason`.

### Run Tests
`cargo test -p node-template-runtime -- --nocapture`

### Run 
```
./build.sh
cargo build --release
./target/release/collateral --dev
```

### UI
https://substrate-ui.parity.io/
Local Node (127.0.0.1:9944)