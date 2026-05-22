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
    LiquidityToken,
    PoolToken,
    InterestStrategy,
    AvailableLiquidity,
    LenderShares(Address),
    Loan(Address),
}

#[contract]
pub struct LendingPool;

#[contractimpl]
impl LendingPool {
    pub fn init(
        env: Env,
        admin: Address,
        liquidity_token: Address,
        pool_token: Address,
        interest_strategy: Address,
    ) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized")
        }
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::LiquidityToken, &liquidity_token);
        env.storage()
            .instance()
            .set(&DataKey::PoolToken, &pool_token);
        env.storage()
            .instance()
            .set(&DataKey::InterestStrategy, &interest_strategy);
        env.storage()
            .instance()
            .set(&DataKey::AvailableLiquidity, &0i128);
    }

    pub fn deposit(env: Env, from: Address, amount: i128) {
        from.require_auth();
        if amount <= 0 {
            panic!("amount must be positive")
        }

        let token = Self::liquidity_token(&env);
        let client = token::Client::new(&env, &token);
        client.transfer(&from, &env.current_contract_address(), &amount);

        let shares_key = DataKey::LenderShares(from.clone());
        let shares: i128 = env.storage().persistent().get(&shares_key).unwrap_or(0i128);
        env.storage().persistent().set(&shares_key, &(shares + amount));

        let pool_token = Self::pool_token(&env);
        let pool_client = token::Client::new(&env, &pool_token);
        pool_client.mint(&from, &amount);

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

        let shares_key = DataKey::LenderShares(to.clone());
        let shares: i128 = env.storage().persistent().get(&shares_key).unwrap_or(0i128);
        if shares < amount {
            panic!("insufficient shares")
        }

        let available = Self::available_liquidity(&env);
        if available < amount {
            panic!("insufficient available liquidity")
        }

        env.storage().persistent().set(&shares_key, &(shares - amount));
        env.storage()
            .instance()
            .set(&DataKey::AvailableLiquidity, &(available - amount));

        let pool_token = Self::pool_token(&env);
        let pool_client = token::Client::new(&env, &pool_token);
        pool_client.burn(&to, &amount);

        let token = Self::liquidity_token(&env);
        let client = token::Client::new(&env, &token);
        client.transfer(&env.current_contract_address(), &to, &amount);
    }

    pub fn originate_loan(
        env: Env,
        borrower: Address,
        amount: i128,
        duration_secs: u64,
        interest_bps: u32,
    ) {
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

        let token = Self::liquidity_token(&env);
        let client = token::Client::new(&env, &token);
        client.transfer(&env.current_contract_address(), &borrower, &amount);
    }

    pub fn repay(env: Env, borrower: Address, amount: i128) {
        borrower.require_auth();
        if amount <= 0 {
            panic!("amount must be positive")
        }

        let key = DataKey::Loan(borrower.clone());
        let mut loan: Loan = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("loan not found"));
        if loan.repaid {
            panic!("loan already repaid")
        }

        let token = Self::liquidity_token(&env);
        let client = token::Client::new(&env, &token);
        client.transfer(&borrower, &env.current_contract_address(), &amount);

        let total_due = Self::amount_due(&loan);
        if amount >= total_due {
            loan.repaid = true;
            env.storage().persistent().set(&key, &loan);
        }

        let available = Self::available_liquidity(&env);
        env.storage()
            .instance()
            .set(&DataKey::AvailableLiquidity, &(available + amount));
    }

    pub fn get_loan(env: Env, borrower: Address) -> Option<Loan> {
        env.storage().persistent().get(&DataKey::Loan(borrower))
    }

    pub fn get_lender_shares(env: Env, lender: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::LenderShares(lender))
            .unwrap_or(0i128)
    }

    pub fn get_available_liquidity(env: Env) -> i128 {
        Self::available_liquidity(&env)
    }

    pub fn get_liquidity_token(env: Env) -> Address {
        Self::liquidity_token(&env)
    }

    pub fn get_pool_token(env: Env) -> Address {
        Self::pool_token(&env)
    }

    pub fn get_interest_strategy(env: Env) -> Address {
        Self::interest_strategy(&env)
    }
}

impl LendingPool {
    fn liquidity_token(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::LiquidityToken)
            .unwrap_or_else(|| panic!("not initialized"))
    }

    fn pool_token(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::PoolToken)
            .unwrap_or_else(|| panic!("not initialized"))
    }

    fn interest_strategy(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::InterestStrategy)
            .unwrap_or_else(|| panic!("not initialized"))
    }

    fn available_liquidity(env: &Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::AvailableLiquidity)
            .unwrap_or(0i128)
    }

    fn amount_due(loan: &Loan) -> i128 {
        let interest = loan
            .principal
            .checked_mul(loan.interest_bps as i128)
            .unwrap_or_else(|| panic!("interest overflow"))
            / 10_000;
        loan.principal + interest
    }
}
