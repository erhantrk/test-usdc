# test-usdc

Minimal NEP-141 fungible token (with NEP-145 storage management and NEP-148 metadata) used as a **mock stablecoin on NEAR testnet**.

- Name: `Test USD Coin (testnet mock)`, symbol: `tUSDC`, decimals: `6`
- Built on `near-sdk =5.30.1` and the standard `near-contract-standards` `FungibleToken`.
- `new(owner_id, total_supply)` mints the whole supply to `owner_id`.
- `mint(account_id, amount)` is owner-only (testnet faucet). The receiver must be registered via `storage_deposit` first.

> **This is a mock token. It has no value. Never deploy it to mainnet or treat it as money.**

## Test

```sh
cargo test
```

## Build

```sh
cargo near build non-reproducible-wasm
# -> target/near/test_usdc.wasm
```

## Deploy (near-cli-rs)

Replace `tusdc.testnet` with your account and `owner.testnet` with the owner.

```sh
near contract deploy tusdc.testnet use-file target/near/test_usdc.wasm \
  with-init-call new json-args '{"owner_id":"owner.testnet","total_supply":"1000000000000000"}' \
  prepaid-gas '100 Tgas' attached-deposit '0 NEAR' \
  network-config testnet sign-with-keychain send

# Register an account (pays storage; ~0.00125 NEAR)
near contract call-function as-transaction tusdc.testnet storage_deposit \
  json-args '{"account_id":"alice.testnet","registration_only":true}' \
  prepaid-gas '30 Tgas' attached-deposit '0.01 NEAR' \
  sign-as alice.testnet network-config testnet sign-with-keychain send

# Faucet mint 1000 tUSDC (6 decimals => 1000000000), owner only
near contract call-function as-transaction tusdc.testnet mint \
  json-args '{"account_id":"alice.testnet","amount":"1000000000"}' \
  prepaid-gas '30 Tgas' attached-deposit '0 NEAR' \
  sign-as owner.testnet network-config testnet sign-with-keychain send
```

Amounts are `U128` strings in base units (1 tUSDC = `1000000`).
