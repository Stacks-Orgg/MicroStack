#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Loan {
    pub principal: i128,
    pub interest_bps: u32,
    pub start_ts: u64,
    pub duration_secs: u64,
    pub repaid: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Token,
    InterestBps,
    AvailableLiquidity,
    Lender(Address),
    Loan(Address),
}

#[contract]
pub struct Microlending;

#[contractimpl]
impl Microlending {
    pub fn init(env: Env, admin: Address, token: Address, interest_bps: u32) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized")
        }
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::InterestBps, &interest_bps);
        env.storage()
            .instance()
            .set(&DataKey::AvailableLiquidity, &0i128);
    }

    pub fn deposit(env: Env, from: Address, amount: i128) {
        from.require_auth();
        if amount <= 0 {
            panic!("amount must be positive")
        }

        let token = Self::token(&env);
        let client = token::Client::new(&env, &token);
        client.transfer(&from, &env.current_contract_address(), &amount);

        let key = DataKey::Lender(from.clone());
        let balance: i128 = env.storage().persistent().get(&key).unwrap_or(0i128);
        env.storage().persistent().set(&key, &(balance + amount));

        let available = Self::available_liquidity(&env);
        env.storage()
            .instance()
            .set(&DataKey::AvailableLiquidity, &(available + amount));
    }

    pub fn withdraw(env: Env, to: Address, amount: i128) {
        to.require_auth();
        if amount <= 0 {
            panic!("amount must be positive")
        }

        let key = DataKey::Lender(to.clone());
        let balance: i128 = env.storage().persistent().get(&key).unwrap_or(0i128);
        if balance < amount {
            panic!("insufficient lender balance")
        }

        let available = Self::available_liquidity(&env);
        if available < amount {
            panic!("insufficient available liquidity")
        }

        env.storage().persistent().set(&key, &(balance - amount));
        env.storage()
            .instance()
            .set(&DataKey::AvailableLiquidity, &(available - amount));

        let token = Self::token(&env);
        let client = token::Client::new(&env, &token);
        client.transfer(&env.current_contract_address(), &to, &amount);
    }

    pub fn take_loan(env: Env, borrower: Address, amount: i128, duration_secs: u64) {
        borrower.require_auth();
        if amount <= 0 {
            panic!("amount must be positive")
        }
        if duration_secs == 0 {
            panic!("duration must be positive")
        }

        if let Some(existing) = env.storage().persistent().get::<_, Loan>(&DataKey::Loan(borrower.clone())) {
            if !existing.repaid {
                panic!("existing loan not repaid")
            }
        }

        let available = Self::available_liquidity(&env);
        if available < amount {
            panic!("insufficient available liquidity")
        }

        let interest_bps = Self::interest_bps(&env);
        let loan = Loan {
            principal: amount,
            interest_bps,
            start_ts: env.ledger().timestamp(),
            duration_secs,
            repaid: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Loan(borrower.clone()), &loan);
        env.storage()
            .instance()
            .set(&DataKey::AvailableLiquidity, &(available - amount));

        let token = Self::token(&env);
        let client = token::Client::new(&env, &token);
        client.transfer(&env.current_contract_address(), &borrower, &amount);
    }

    pub fn repay(env: Env, borrower: Address) -> i128 {
        borrower.require_auth();

        let key = DataKey::Loan(borrower.clone());
        let mut loan: Loan = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("loan not found"));
        if loan.repaid {
            panic!("loan already repaid")
        }

        let due = Self::amount_due(&env, &loan);

        let token = Self::token(&env);
        let client = token::Client::new(&env, &token);
        client.transfer(&borrower, &env.current_contract_address(), &due);

        loan.repaid = true;
        env.storage().persistent().set(&key, &loan);

        let available = Self::available_liquidity(&env);
        env.storage()
            .instance()
            .set(&DataKey::AvailableLiquidity, &(available + due));

        due
    }

    pub fn get_loan(env: Env, borrower: Address) -> Option<Loan> {
        env.storage().persistent().get(&DataKey::Loan(borrower))
    }

    pub fn get_lender_balance(env: Env, lender: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Lender(lender))
            .unwrap_or(0i128)
    }

    pub fn get_available_liquidity(env: Env) -> i128 {
        Self::available_liquidity(&env)
    }

    pub fn get_interest_bps(env: Env) -> u32 {
        Self::interest_bps(&env)
    }

    pub fn get_token(env: Env) -> Address {
        Self::token(&env)
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("not initialized"))
    }
}

impl Microlending {
    fn token(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic!("not initialized"))
    }

    fn interest_bps(env: &Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::InterestBps)
            .unwrap_or_else(|| panic!("not initialized"))
    }

    fn available_liquidity(env: &Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::AvailableLiquidity)
            .unwrap_or(0i128)
    }

    fn amount_due(_env: &Env, loan: &Loan) -> i128 {
        let interest = loan
            .principal
            .checked_mul(loan.interest_bps as i128)
            .unwrap_or_else(|| panic!("interest overflow"))
            / 10_000;
        loan.principal + interest
    }
}
