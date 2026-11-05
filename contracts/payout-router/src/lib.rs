#![no_std]
//! Fist Commerce PayoutRouter — Track B two-step fund → payout + repay (mirrors EVM PayoutRouter).

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, BytesN, Env,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    NotFound = 4,
    AlreadyPaid = 5,
    InvalidAmount = 6,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRequest {
    pub receivable_id: BytesN<32>,
    pub merchant: Address,
    pub amount: i128,
    pub is_paid_out: bool,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Initialized,
    Admin,
    FundingPool,
    SettlementAsset,
    Allocator,
    MinRepayment,
    Request(BytesN<32>),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceivablePayoutRequested {
    #[topic]
    pub receivable_id: BytesN<32>,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceivablePaidOut {
    #[topic]
    pub receivable_id: BytesN<32>,
    pub merchant: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceivablePayoutCancelled {
    #[topic]
    pub receivable_id: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceivableRepayment {
    #[topic]
    pub receivable_id: BytesN<32>,
    pub payer: Address,
    pub amount: i128,
}

#[contract]
pub struct PayoutRouter;

#[contractimpl]
impl PayoutRouter {
    pub fn initialize(
        env: Env,
        admin: Address,
        funding_pool: Address,
        settlement_asset: Address,
        allocator: Address,
        min_repayment: i128,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::FundingPool, &funding_pool);
        env.storage()
            .instance()
            .set(&DataKey::SettlementAsset, &settlement_asset);
        env.storage().instance().set(&DataKey::Allocator, &allocator);
        env.storage()
            .instance()
            .set(&DataKey::MinRepayment, &min_repayment);
        env.storage().instance().set(&DataKey::Initialized, &true);
        Ok(())
    }

    pub fn create_receivable_payout_request(
        env: Env,
        receivable_id: BytesN<32>,
        merchant: Address,
        amount: i128,
    ) -> Result<(), Error> {
        let pool: Address = env
            .storage()
            .instance()
            .get(&DataKey::FundingPool)
            .ok_or(Error::NotInitialized)?;
        pool.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let req = PayoutRequest {
            receivable_id: receivable_id.clone(),
            merchant,
            amount,
            is_paid_out: false,
            timestamp: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Request(receivable_id.clone()), &req);
        ReceivablePayoutRequested {
            receivable_id,
            amount,
        }
        .publish(&env);
        Ok(())
    }

    pub fn cancel_receivable_payout_request(env: Env, receivable_id: BytesN<32>) -> Result<(), Error> {
        let pool: Address = env
            .storage()
            .instance()
            .get(&DataKey::FundingPool)
            .ok_or(Error::NotInitialized)?;
        pool.require_auth();
        let req: PayoutRequest = env
            .storage()
            .persistent()
            .get(&DataKey::Request(receivable_id.clone()))
            .ok_or(Error::NotFound)?;
        if req.is_paid_out {
            return Err(Error::AlreadyPaid);
        }
        env.storage()
            .persistent()
            .remove(&DataKey::Request(receivable_id.clone()));
        ReceivablePayoutCancelled { receivable_id }.publish(&env);
        Ok(())
    }

    pub fn payout_receivable(env: Env, receivable_id: BytesN<32>) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        let mut req: PayoutRequest = env
            .storage()
            .persistent()
            .get(&DataKey::Request(receivable_id.clone()))
            .ok_or(Error::NotFound)?;
        if req.is_paid_out {
            return Err(Error::AlreadyPaid);
        }
        let pool: Address = env.storage().instance().get(&DataKey::FundingPool).unwrap();
        let asset: Address = env
            .storage()
            .instance()
            .get(&DataKey::SettlementAsset)
            .unwrap();
        let token = token::Client::new(&env, &asset);
        token.transfer(&pool, &req.merchant, &req.amount);
        req.is_paid_out = true;
        env.storage()
            .persistent()
            .set(&DataKey::Request(receivable_id.clone()), &req);
        ReceivablePaidOut {
            receivable_id,
            merchant: req.merchant,
            amount: req.amount,
        }
        .publish(&env);
        Ok(())
    }

    pub fn repay_receivable(
        env: Env,
        receivable_id: BytesN<32>,
        payer: Address,
        amount: i128,
    ) -> Result<(), Error> {
        payer.require_auth();
        let min: i128 = env
            .storage()
            .instance()
            .get(&DataKey::MinRepayment)
            .unwrap_or(1);
        if amount < min {
            return Err(Error::InvalidAmount);
        }
        let pool: Address = env.storage().instance().get(&DataKey::FundingPool).unwrap();
        let asset: Address = env
            .storage()
            .instance()
            .get(&DataKey::SettlementAsset)
            .unwrap();
        let token = token::Client::new(&env, &asset);
        token.transfer(&payer, &pool, &amount);
        ReceivableRepayment {
            receivable_id,
            payer,
            amount,
        }
        .publish(&env);
        Ok(())
    }

    pub fn get_request(env: Env, receivable_id: BytesN<32>) -> Option<PayoutRequest> {
        env.storage()
            .persistent()
            .get(&DataKey::Request(receivable_id))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::Address as _,
        token::{StellarAssetClient, TokenClient},
        Env,
    };

    #[test]
    fn two_step_payout_and_repay() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let pool = Address::generate(&env);
        let merchant = Address::generate(&env);
        let allocator = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let token = sac.address();
        let token_admin = StellarAssetClient::new(&env, &token);
        let token_client = TokenClient::new(&env, &token);
        token_admin.mint(&pool, &10_000);

        let id = env.register(PayoutRouter, ());
        let c = PayoutRouterClient::new(&env, &id);
        c.initialize(&admin, &pool, &token, &allocator, &1);
        let rid = BytesN::from_array(&env, &[9u8; 32]);
        c.create_receivable_payout_request(&rid, &merchant, &5_000);
        c.payout_receivable(&rid);
        assert_eq!(token_client.balance(&merchant), 5_000);
        token_admin.mint(&merchant, &5_000);
        c.repay_receivable(&rid, &merchant, &5_000);
    }
}
