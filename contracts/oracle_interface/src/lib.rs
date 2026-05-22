#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub decimals: u32,
    pub updated_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Price,
}

#[contract]
pub struct OracleInterface;

#[contractimpl]
impl OracleInterface {
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized")
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn set_price(env: Env, admin: Address, price: i128, decimals: u32) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("not initialized"));
        if admin != stored_admin {
            panic!("not authorized")
        }

        let data = PriceData {
            price,
            decimals,
            updated_at: env.ledger().timestamp(),
        };
        env.storage().instance().set(&DataKey::Price, &data);
    }

    pub fn get_price(env: Env) -> PriceData {
        env.storage()
            .instance()
            .get(&DataKey::Price)
            .unwrap_or_else(|| panic!("price not available"))
    }
}
