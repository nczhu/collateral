# Simple Collateral

A implementation of tokenized debt. A token owner can collateralize a token for a loan, and pay back with interest. Creditors can seize the collateral on defaulted loans.

Implementation uses balances module, ERC721. 


### Run 
```
./build.sh
cargo build
./release...
```

### Run Tests
`cargo test -p node-template-runtime -- --nocapture`