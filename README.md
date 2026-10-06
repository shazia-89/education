# Superteam KZ — Assignment 3: Secure Escrow

## Overview

This workspace contains the completed Level 1 token work and a new `programs/escrow` Anchor program for Assignment 3. The escrow locks Token-2022 tokens in a per-deal PDA vault until the sender releases or cancels the deal.

## Architecture

- `EscrowState` stores sender, receiver, mint, amount, `deal_id`, PDA bump, and status.
- Every deal uses the unique PDA seeds `[b"escrow", sender, deal_id]`.
- The vault is the Token-2022 associated token account owned by that deal PDA and constrained to the stored mint/token program.
- State transitions are `Created -> Funded -> Released` or `Created -> Funded -> Cancelled`.
- Release and cancel are final operations: they transfer the exact escrow amount, close the empty vault, and close the state account back to the sender.

## Instructions

1. `initialize`: creates a non-zero deal for a different receiver and its unique PDA/vault.
2. `deposit`: sender deposits exactly the stored amount with `transfer_checked`.
3. `release`: sender releases a funded deal to the receiver token account.
4. `cancel`: sender cancels a funded deal and receives the deposited tokens back.

## Security model

All critical invariants are enforced by Anchor account constraints and checked Token-2022 CPI:

- signer/authority validation
- unique PDA seed and bump validation
- stored mint validation
- token-account mint, authority, and token-program validation
- exact escrow amount
- non-zero amount and distinct sender/receiver
- explicit state validation preventing repeated finalization
- PDA signer seeds for vault transfers and closure

## Threat model

The program is designed to prevent unauthorized release/cancel, wrong-mint deposits, wrong token-program/token-account substitution, duplicate finalization, and partial state transitions. Failed instructions return before mutating escrow state; successful finalization closes the state and empty vault.

## Build and test

```bash
anchor build --ignore-keys
cargo test --workspace --locked
```

The fixed course stack is Anchor 1.1.2, Solana CLI 3.1.10, Rust 1.89.0, LiteSVM 0.10.0, Token-2022 and `anchor_spl::token_interface`.

Never commit private keys, seed phrases, wallet JSON files, or RPC secrets.
