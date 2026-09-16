# Anchor AMM

A constant-product automated market maker built with the [Anchor](https://www.anchor-lang.com/) framework on Solana. The program lets anyone create a pool for a pair of SPL tokens, seed it with liquidity, swap between the two assets, and redeem their share of the reserves.

Liquidity is held in program-owned vaults. Prices are not quoted by an order book — they fall out of the curve:

```
x * y = k
```

where `x` and `y` are the vault reserves. Any swap must leave `k` unchanged (before fees), so the larger a trade is relative to the pool, the worse the price it receives. Slippage is a property of the math, not a separate mechanism.

---

## Features

- **Permissionless pool creation** — each pool is keyed by a `seed`, so one deployment hosts many independent markets.
- **Constant-product swaps** with a configurable fee in basis points.
- **LP tokens** minted proportionally on deposit and burned on withdrawal, representing a claim on the reserves.
- **Slippage protection** on every user-facing instruction (`min_out`, `max_x`/`max_y`, `min_x`/`min_y`).
- **Pausable pools** via a `locked` flag on the config account.
- **Checked integer math** throughout — `u128` intermediates, no floating point, rounding always in the pool's favour.

---

## Repository layout

```
programs/amm/
└── src/
    ├── instructions/
    │   ├── initialize.rs     # create config, LP mint and both vaults
    │   ├── deposit.rs        # add liquidity, mint LP tokens
    │   ├── swap.rs           # constant-product swap in either direction
    │   └── withdraw.rs       # burn LP tokens, return proportional reserves
    ├── instructions.rs       # module re-exports
    ├── state.rs              # Config account
    ├── constants.rs          # seeds and shared constants
    ├── error.rs              # custom AmmError variants
    └── lib.rs                # program entrypoint and handlers

tests/
├── ix_handlers/
│   ├── init.rs
│   ├── deposit.rs
│   ├── swap.rs
│   ├── withdraw.rs
│   └── mod.rs
└── test.rs                   # test harness entrypoint
```

---

## Accounts

| Account | Type | Notes |
| --- | --- | --- |
| `config` | PDA | Pool state. Seeds: `[b"config", seed.to_le_bytes()]` |
| `mint_lp` | PDA mint | Mint authority is `config` |
| `vault_x` | ATA | Owned by `config`, holds token X reserves |
| `vault_y` | ATA | Owned by `config`, holds token Y reserves |

### `Config`

```rust
#[account]
pub struct Config {
    pub seed: u64,
    pub authority: Option<Pubkey>,
    pub mint_x: Pubkey,
    pub mint_y: Pubkey,
    pub fee: u16,          // basis points
    pub locked: bool,
    pub config_bump: u8,
    pub lp_bump: u8,
}
```

Because the vaults are owned by the `config` PDA, tokens can only leave a pool through a CPI signed by the program itself:

```rust
let seeds = &[
    b"config",
    &self.config.seed.to_le_bytes()[..],
    &[self.config.config_bump],
];
let signer_seeds = &[&seeds[..]];
```

---

## Instructions

### `initialize(seed, fee, authority)`

Creates the config account, the LP mint and both token vaults. Sets the fee and the optional authority that may lock the pool.

### `deposit(amount, max_x, max_y)`

`amount` is the quantity of LP tokens the depositor wants to receive. The program derives how much of each token that costs.

- **First deposit** — the ratio is unconstrained; the depositor sets the pool's opening price. LP supply is initialised from the geometric mean of the two amounts.
- **Subsequent deposits** — amounts must track the current reserve ratio:

```
x = (amount * reserve_x) / lp_supply
y = (amount * reserve_y) / lp_supply
```

The transaction fails if `x > max_x` or `y > max_y`.

### `swap(is_x, amount_in, min_out)`

`is_x` selects the direction. The fee is deducted from the input, then the output is solved from the curve:

```
amount_out = (reserve_out * amount_in_after_fee) / (reserve_in + amount_in_after_fee)
```

The transaction fails if `amount_out < min_out`, which is what makes a swap safe to submit into a public mempool.

### `withdraw(amount, min_x, min_y)`

Burns `amount` LP tokens and returns the proportional share of each reserve, subject to the two minimums.

---

## Errors

| Error | Raised when |
| --- | --- |
| `PoolLocked` | The pool's `locked` flag is set |
| `SlippageExceeded` | Output below `min_out`, or input above `max_x`/`max_y` |
| `InvalidAmount` | A zero amount was supplied |
| `Overflow` | A checked arithmetic operation failed |
| `InvalidMint` | Supplied mint does not match the pool's configuration |
| `NoLiquidity` | Swap or withdraw attempted against an empty pool |

---

## Getting started

### Prerequisites

- Rust (version pinned in `rust-toolchain.toml`)
- Solana CLI
- Anchor CLI
- Node.js and Yarn

### Build

```bash
anchor build
```

### Test

The test suite lives in a Rust crate under `tests/`, with one module per instruction handler:

```bash
cargo test
```

Or run the full Anchor pipeline:

```bash
anchor test
```

### Deploy to devnet

```bash
solana config set --url devnet
anchor deploy --provider.cluster devnet
```

After the first deploy, update `declare_id!` in `lib.rs` and the `[programs.devnet]` entry in `Anchor.toml` with the deployed program ID, then rebuild.

---

## Test results

All instruction handlers are covered: pool initialisation, first and subsequent deposits, swaps in both directions, and withdrawals.

![All tests passing](./img/amm%20test%20Screenshot.png)

---

## Program ID

| Cluster | Address |
| --- | --- |
| Devnet | `<program id>` |

---

## Notes on the math

Three details matter more than they look:

1. **Multiply before dividing.** Integer division truncates, so dividing first silently throws away precision.
2. **Widen to `u128` for intermediates.** `reserve_out * amount_in` overflows `u64` at realistic reserve sizes.
3. **Round toward the pool.** Any fraction of a token that rounds toward the user is a value leak that can be repeated in a loop.

---

## License

MIT