#![no_std]
//! Fist Commerce ReceivableAllocator — Track B (mirrors AllocationController).
//! Risk tiers, exposure caps, EVM-like receivable lifecycle for gated FE ops.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, BytesN, Env, String,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    NotFound = 4,
    InvalidStatus = 5,
    TierInactive = 6,
    ExposureExceeded = 7,
    TenorExpired = 8,
    InvalidInput = 9,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    Created = 1,
    Verified = 2,
    Funded = 3,
    PaidOut = 4,
    Repaid = 5,
    Defaulted = 6,
    Rejected = 7,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskTier {
    pub tenor_rate_bps: u32,
    pub max_tenor_days: u32,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receivable {
    pub id: BytesN<32>,
    pub token_id: u64,
    pub approval_time: u64,
    pub merchant_id: BytesN<32>,
    pub merchant: Address,
    pub risk_tier_id: u32,
    pub funded_amount: i128,
    pub expected_repayment: i128,
    pub repaid_amount: i128,
    pub remaining_principal: i128,
    pub status: Status,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Initialized,
    Admin,
    Servicer,
    Pool,
    MaxMerchantBps,
    RiskTier(u32),
    Receivable(BytesN<32>),
    MerchantExposure(BytesN<32>),
    TotalOutstanding,
    ProtocolController,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceivableCreated {
    #[topic]
    pub id: BytesN<32>,
    pub merchant: Address,
    pub funded_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceivableVerified {
    #[topic]
    pub id: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentDefaulted {
    #[topic]
    pub id: BytesN<32>,
}

#[contract]
pub struct ReceivableAllocator;

#[contractimpl]
impl ReceivableAllocator {
    pub fn initialize(
        env: Env,
        admin: Address,
        servicer: Address,
        max_merchant_bps: u32,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Servicer, &servicer);
        env.storage()
            .instance()
            .set(&DataKey::MaxMerchantBps, &max_merchant_bps);
        env.storage().instance().set(&DataKey::TotalOutstanding, &0i128);
        env.storage().instance().set(&DataKey::Initialized, &true);
        Ok(())
    }

    pub fn set_funding_pool(env: Env, pool: Address) -> Result<(), Error> {
        Self::require_admin(&env)?;
        env.storage().instance().set(&DataKey::Pool, &pool);
        Ok(())
    }

    pub fn set_risk_tier(
        env: Env,
        tier_id: u32,
        tenor_rate_bps: u32,
        max_tenor_days: u32,
        active: bool,
    ) -> Result<(), Error> {
        Self::require_admin(&env)?;
        env.storage().persistent().set(
            &DataKey::RiskTier(tier_id),
            &RiskTier {
                tenor_rate_bps,
                max_tenor_days,
                active,
            },
        );
        Ok(())
    }

    pub fn create_receivable(
        env: Env,
        token_id: u64,
        merchant_id: BytesN<32>,
        merchant: Address,
        risk_tier_id: u32,
        funded_amount: i128,
    ) -> Result<BytesN<32>, Error> {
        Self::require_servicer(&env)?;
        if funded_amount <= 0 {
            return Err(Error::InvalidInput);
        }
        let tier: RiskTier = env
            .storage()
            .persistent()
            .get(&DataKey::RiskTier(risk_tier_id))
            .ok_or(Error::TierInactive)?;
        if !tier.active {
            return Err(Error::TierInactive);
        }
        let interest = funded_amount * (tier.tenor_rate_bps as i128) / 10_000;
        let expected = funded_amount + interest;
        // id = sha256(merchant_id || token_id)
        let mut buf = [0u8; 40];
        let mid = merchant_id.to_array();
        buf[..32].copy_from_slice(&mid);
        buf[32..40].copy_from_slice(&token_id.to_be_bytes());
        let id = env.crypto().sha256(&soroban_sdk::Bytes::from_slice(&env, &buf));
        let id = BytesN::from_array(&env, &id.to_array());

        if env.storage().persistent().has(&DataKey::Receivable(id.clone())) {
            return Err(Error::InvalidInput);
        }
        let rec = Receivable {
            id: id.clone(),
            token_id,
            approval_time: 0,
            merchant_id: merchant_id.clone(),
            merchant: merchant.clone(),
            risk_tier_id,
            funded_amount,
            expected_repayment: expected,
            repaid_amount: 0,
            remaining_principal: 0,
            status: Status::Created,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Receivable(id.clone()), &rec);
        ReceivableCreated {
            id: id.clone(),
            merchant,
            funded_amount,
        }
        .publish(&env);
        Ok(id)
    }

    pub fn mark_verified(env: Env, id: BytesN<32>) -> Result<(), Error> {
        Self::require_servicer(&env)?;
        let mut rec: Receivable = env
            .storage()
            .persistent()
            .get(&DataKey::Receivable(id.clone()))
            .ok_or(Error::NotFound)?;
        if rec.status != Status::Created {
            return Err(Error::InvalidStatus);
        }
        rec.status = Status::Verified;
        rec.approval_time = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&DataKey::Receivable(id.clone()), &rec);
        ReceivableVerified { id }.publish(&env);
        Ok(())
    }

    pub fn can_fund(env: Env, id: BytesN<32>) -> (bool, String) {
        let Some(rec) = env
            .storage()
            .persistent()
            .get::<_, Receivable>(&DataKey::Receivable(id))
        else {
            return (false, String::from_str(&env, "not_found"));
        };
        if rec.status != Status::Verified {
            return (false, String::from_str(&env, "bad_status"));
        }
        let Some(tier) = env
            .storage()
            .persistent()
            .get::<_, RiskTier>(&DataKey::RiskTier(rec.risk_tier_id))
        else {
            return (false, String::from_str(&env, "no_tier"));
        };
        if !tier.active {
            return (false, String::from_str(&env, "tier_inactive"));
        }
        let deadline = rec.approval_time + (tier.max_tenor_days as u64) * 86_400;
        if env.ledger().timestamp() > deadline {
            return (false, String::from_str(&env, "tenor_expired"));
        }
        let max_bps: u32 = env
            .storage()
            .instance()
            .get(&DataKey::MaxMerchantBps)
            .unwrap_or(10_000);
        let total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalOutstanding)
            .unwrap_or(0);
        let exp: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::MerchantExposure(rec.merchant_id))
            .unwrap_or(0);
        if total > 0 {
            let next = exp + rec.funded_amount;
            if next * 10_000 > total * (max_bps as i128) {
                return (false, String::from_str(&env, "exposure"));
            }
        }
        (true, String::from_str(&env, "ok"))
    }

    pub fn record_funding(env: Env, id: BytesN<32>) -> Result<(), Error> {
        Self::require_pool(&env)?;
        let mut rec: Receivable = env
            .storage()
            .persistent()
            .get(&DataKey::Receivable(id.clone()))
            .ok_or(Error::NotFound)?;
        if rec.status != Status::Verified {
            return Err(Error::InvalidStatus);
        }
        let (ok, _) = Self::can_fund(env.clone(), id.clone());
        if !ok {
            return Err(Error::ExposureExceeded);
        }
        rec.status = Status::Funded;
        rec.remaining_principal = rec.funded_amount;
        let mid = rec.merchant_id.clone();
        let amt = rec.funded_amount;
        env.storage()
            .persistent()
            .set(&DataKey::Receivable(id), &rec);
        let exp: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::MerchantExposure(mid.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::MerchantExposure(mid), &(exp + amt));
        let total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalOutstanding)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalOutstanding, &(total + amt));
        Ok(())
    }

    pub fn record_paid_out(env: Env, id: BytesN<32>) -> Result<(), Error> {
        Self::require_pool(&env)?;
        let mut rec: Receivable = env
            .storage()
            .persistent()
            .get(&DataKey::Receivable(id.clone()))
            .ok_or(Error::NotFound)?;
        if rec.status != Status::Funded {
            return Err(Error::InvalidStatus);
        }
        rec.status = Status::PaidOut;
        env.storage()
            .persistent()
            .set(&DataKey::Receivable(id), &rec);
        Ok(())
    }

    pub fn record_repayment(env: Env, id: BytesN<32>, amount: i128) -> Result<(), Error> {
        Self::require_pool(&env)?;
        let mut rec: Receivable = env
            .storage()
            .persistent()
            .get(&DataKey::Receivable(id.clone()))
            .ok_or(Error::NotFound)?;
        if rec.status != Status::PaidOut && rec.status != Status::Funded {
            return Err(Error::InvalidStatus);
        }
        rec.repaid_amount += amount;
        let to_prin = if amount <= rec.remaining_principal {
            amount
        } else {
            rec.remaining_principal
        };
        rec.remaining_principal -= to_prin;
        if rec.repaid_amount >= rec.expected_repayment {
            rec.status = Status::Repaid;
        }
        env.storage()
            .persistent()
            .set(&DataKey::Receivable(id), &rec);
        Ok(())
    }

    pub fn mark_defaulted(env: Env, id: BytesN<32>) -> Result<(), Error> {
        Self::require_pool(&env)?;
        let mut rec: Receivable = env
            .storage()
            .persistent()
            .get(&DataKey::Receivable(id.clone()))
            .ok_or(Error::NotFound)?;
        rec.status = Status::Defaulted;
        env.storage()
            .persistent()
            .set(&DataKey::Receivable(id.clone()), &rec);
        PaymentDefaulted { id }.publish(&env);
        Ok(())
    }

    pub fn get_receivable(env: Env, id: BytesN<32>) -> Option<Receivable> {
        env.storage().persistent().get(&DataKey::Receivable(id))
    }

    pub fn get_risk_tier(env: Env, tier_id: u32) -> Option<RiskTier> {
        env.storage().persistent().get(&DataKey::RiskTier(tier_id))
    }
}

impl ReceivableAllocator {
    fn require_admin(env: &Env) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        Ok(())
    }

    fn require_servicer(env: &Env) -> Result<(), Error> {
        let s: Address = env
            .storage()
            .instance()
            .get(&DataKey::Servicer)
            .ok_or(Error::NotInitialized)?;
        s.require_auth();
        Ok(())
    }

    fn require_pool(env: &Env) -> Result<(), Error> {
        let p: Address = env
            .storage()
            .instance()
            .get(&DataKey::Pool)
            .ok_or(Error::NotInitialized)?;
        p.require_auth();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn create_verify_risk_tier() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let servicer = Address::generate(&env);
        let merchant = Address::generate(&env);
        let id = env.register(ReceivableAllocator, ());
        let c = ReceivableAllocatorClient::new(&env, &id);
        c.initialize(&admin, &servicer, &2_500);
        c.set_risk_tier(&1, &500, &90, &true);
        let mid = BytesN::from_array(&env, &[1u8; 32]);
        let rid = c.create_receivable(&1, &mid, &merchant, &1, &10_000);
        c.mark_verified(&rid);
        let (ok, _) = c.can_fund(&rid);
        assert!(ok);
        let rec = c.get_receivable(&rid).unwrap();
        assert_eq!(rec.expected_repayment, 10_500); // 5% of 10000
    }
}
