#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollateralPosition {
    pub amount: i128,
    pub owner: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    CollateralToken,
    Oracle,
    LiquidationBps,
    Position(Address),
}

#[contract]
pub struct CollateralLiquidation;

#[contractimpl]
impl CollateralLiquidation {
    pub fn init(
        env: Env,
        admin: Address,
        collateral_token: Address,
        oracle: Address,
        liquidation_bps: u32,
    ) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized")
        }
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::CollateralToken, &collateral_token);
        env.storage().instance().set(&DataKey::Oracle, &oracle);
        env.storage()
            .instance()
            .set(&DataKey::LiquidationBps, &liquidation_bps);
    }

    pub fn lock(env: Env, owner: Address, amount: i128) {
        owner.require_auth();
        if amount <= 0 {
            panic!("amount must be positive")
        }

        let token = Self::collateral_token(&env);
        let client = token::Client::new(&env, &token);
        client.transfer(&owner, &env.current_contract_address(), &amount);

        let key = DataKey::Position(owner.clone());
        let existing: CollateralPosition = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(CollateralPosition {
                amount: 0,
                owner: owner.clone(),
            });
        env.storage().persistent().set(
            &key,
            &CollateralPosition {
                amount: existing.amount + amount,
                owner,
            },
        );
    }

    pub fn unlock(env: Env, owner: Address, amount: i128) {
        owner.require_auth();
        if amount <= 0 {
            panic!("amount must be positive")
        }

        let key = DataKey::Position(owner.clone());
        let mut position: CollateralPosition = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("position not found"));
        if position.amount < amount {
            panic!("insufficient collateral")
        }

        position.amount -= amount;
        env.storage().persistent().set(&key, &position);

        let token = Self::collateral_token(&env);
        let client = token::Client::new(&env, &token);
        client.transfer(&env.current_contract_address(), &owner, &amount);
    }

    pub fn liquidate(env: Env, owner: Address, liquidator: Address, repay_amount: i128) {
        liquidator.require_auth();
        if repay_amount <= 0 {
            panic!("amount must be positive")
        }

        let key = DataKey::Position(owner.clone());
        let position: CollateralPosition = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("position not found"));

        let discount_bps = Self::liquidation_bps(&env);
        let discounted_amount = repay_amount
            .checked_mul(10_000 - discount_bps as i128)
            .unwrap_or_else(|| panic!("overflow"))
            / 10_000;

        let seize = if discounted_amount > position.amount {
            position.amount
        } else {
            discounted_amount
        };

        let token = Self::collateral_token(&env);
        let client = token::Client::new(&env, &token);
        client.transfer(&env.current_contract_address(), &liquidator, &seize);

        let remaining = position.amount - seize;
        if remaining == 0 {
            env.storage().persistent().remove(&key);
        } else {
            env.storage()
                .persistent()
                .set(&key, &CollateralPosition { amount: remaining, owner });
        }
    }

    pub fn get_position(env: Env, owner: Address) -> Option<CollateralPosition> {
        env.storage().persistent().get(&DataKey::Position(owner))
    }
}

impl CollateralLiquidation {
    fn collateral_token(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::CollateralToken)
            .unwrap_or_else(|| panic!("not initialized"))
    }

    fn liquidation_bps(env: &Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::LiquidationBps)
            .unwrap_or_else(|| panic!("not initialized"))
    }
}
