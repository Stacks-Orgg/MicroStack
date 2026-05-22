#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateParams {
    pub base_bps: u32,
    pub slope_bps: u32,
    pub optimal_util_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Params,
}

#[contract]
pub struct InterestStrategy;

#[contractimpl]
impl InterestStrategy {
    pub fn init(env: Env, params: RateParams) {
        if env.storage().instance().has(&DataKey::Params) {
            panic!("already initialized")
        }
        env.storage().instance().set(&DataKey::Params, &params);
    }

    pub fn current_rate_bps(env: Env, utilization_bps: u32) -> u32 {
        let params: RateParams = env
            .storage()
            .instance()
            .get(&DataKey::Params)
            .unwrap_or_else(|| panic!("not initialized"));

        if utilization_bps <= params.optimal_util_bps {
            params.base_bps
                + (params.slope_bps * utilization_bps / params.optimal_util_bps.max(1))
        } else {
            params.base_bps + params.slope_bps
        }
    }

    pub fn accrue(principal: i128, rate_bps: u32, elapsed_secs: u64) -> i128 {
        if principal <= 0 {
            return 0;
        }
        let interest = principal
            .checked_mul(rate_bps as i128)
            .unwrap_or_else(|| panic!("overflow"))
            .checked_mul(elapsed_secs as i128)
            .unwrap_or_else(|| panic!("overflow"))
            / 10_000
            / (365 * 24 * 60 * 60);
        principal + interest
    }
}
