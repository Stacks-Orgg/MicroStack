# MicroStack

## Overview
This repository builds a modular microlending platform for the Stellar Soroban smart contract stack. Microlending introduces the risk of default, so the contracts are designed to handle collateral, interest accrual, and liquidations without a single monolithic contract.

## Core contracts
The system is composed of small, focused contracts that interact with each other:

1. Lending Pool Contract (The Core)
	- Accepts USDC deposits from lenders and allows borrowers to withdraw loans from pooled liquidity.
	- Mints pool shares (e.g., lUSDC) to represent a lender’s proportional ownership of the pool plus accrued interest.
	- Records borrower loans with principal, start timestamp, duration, and interest rate.
	- Contract: [contracts/lending_pool/src/lib.rs](contracts/lending_pool/src/lib.rs)

2. Collateral & Liquidation Contract (The Enforcer)
	- Escrows collateral (e.g., XLM or another SEP-41 asset) on behalf of the borrower.
	- Tracks collateral health and allows liquidations if collateral value falls below a safe threshold or loan duration expires.
	- Enables liquidators to repay debt in exchange for collateral at a discount.
	- Contract: [contracts/collateral_liquidation/src/lib.rs](contracts/collateral_liquidation/src/lib.rs)

3. Interest Rate Strategy Contract (The Calculator)
	- Calculates dynamic interest rates based on utilization, encouraging liquidity when the pool is heavily borrowed.
	- Accrues interest over time using ledger timestamps.
	- Contract: [contracts/interest_strategy/src/lib.rs](contracts/interest_strategy/src/lib.rs)

4. Oracle Interface (The Price Feed)
	- Provides a price feed for collateral valuation in USD terms.
	- Used by the collateral contract to enforce healthy collateral ratios and trigger liquidations.
	- Contract: [contracts/oracle_interface/src/lib.rs](contracts/oracle_interface/src/lib.rs)

5. SEP-41 Token Interfaces
	- All token transfers use Soroban’s SEP-41 compatible token interface for USDC (liquidity) and collateral assets.
	- `require_auth()` is used to enforce user signatures for deposits, withdrawals, and repayments.

## Project layout
- [contracts/lending_pool/src/lib.rs](contracts/lending_pool/src/lib.rs)
- [contracts/collateral_liquidation/src/lib.rs](contracts/collateral_liquidation/src/lib.rs)
- [contracts/interest_strategy/src/lib.rs](contracts/interest_strategy/src/lib.rs)
- [contracts/oracle_interface/src/lib.rs](contracts/oracle_interface/src/lib.rs)

## Note on prior monolithic prototype
The earlier single-contract prototype is kept only as a reference and is not the recommended deployment path. The modular contracts above are the intended design for production.