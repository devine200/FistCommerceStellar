#![no_std]
//! Fist Commerce TradeCreditPool — Track A central financing path.
//! Settlement asset is a Stellar Asset Contract (e.g. FISTUSD SAC), never branded as USDC.

use attestation_registry::{AttestationRegistryClient, AttestationType};
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, BytesN,
    Env, String,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvestorNotApproved = 4,
    InvalidAmount = 5,
    OperationIdReused = 6,
    ReceivableExists = 7,
    ReceivableNotFound = 8,
    InvalidEconomics = 9,
    InvalidStatus = 10,
    MaturityNotFuture = 11,
    AttestationMissing = 12,
    InsufficientLiquidity = 13,
    OverRepayment = 14,
    TransferFailed = 15,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceivableStatus {
    Registered = 1,
    Approved = 2,
    Funded = 3,
    PartiallyRepaid = 4,
    Settled = 5,
    Cancelled = 6,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receivable {
    pub receivable_id: String,
    pub merchant_id: String,
    pub merchant_payout_address: Address,
    pub advance_amount: i128,
    pub contractual_repayment_amount: i128,
    pub principal_outstanding: i128,
    pub total_repaid: i128,
    pub realized_income: i128,
    pub maturity_timestamp: u64,
    pub status: ReceivableStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolAccounting {
    pub total_deposited: i128,
    pub total_available: i128,
    pub total_deployed_principal: i128,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Initialized,
    Admin,
    PoolManager,
    Originator,
    RiskManager,
    SettlementAsset,
    AttestationRegistry,
    InvestorStatus(Address),
    InvestorUnits(Address),
    TotalDeposited,
    TotalAvailable,
    TotalDeployedPrincipal,
    Receivable(String),
    UsedOperationId(BytesN<32>),
    // Track B extensions (optional wiring)
    ProtocolController,
    WithdrawalDuration,
    WithdrawalGap,
    MinDeposit,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvestorStatusSet {
    #[topic]
    pub investor: Address,
    pub approved: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deposited {
    #[topic]
    pub investor: Address,
    pub amount: i128,
    pub operation_id: BytesN<32>,
    pub units: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceivableRegistered {
    #[topic]
    pub receivable_id: String,
    pub merchant_id: String,
    pub advance_amount: i128,
    pub contractual_repayment_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceivableApproved {
    #[topic]
    pub receivable_id: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceivableFunded {
    #[topic]
    pub receivable_id: String,
    pub amount: i128,
    pub merchant: Address,
    pub operation_id: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceivableDisbursed {
    #[topic]
    pub receivable_id: String,
    pub merchant: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Repaid {
    #[topic]
    pub receivable_id: String,
    pub payer: Address,
    pub amount: i128,
    pub total_repaid: i128,
    pub status: ReceivableStatus,
    pub operation_id: BytesN<32>,
}

#[contract]
pub struct TradeCreditPool;

#[contractimpl]
impl TradeCreditPool {
    pub fn initialize(
        env: Env,
        admin: Address,
        pool_manager: Address,
        originator: Address,
        risk_manager: Address,
        settlement_asset: Address,
        attestation_registry: Address,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::PoolManager, &pool_manager);
        env.storage().instance().set(&DataKey::Originator, &originator);
        env.storage().instance().set(&DataKey::RiskManager, &risk_manager);
        env.storage()
            .instance()
            .set(&DataKey::SettlementAsset, &settlement_asset);
        env.storage()
            .instance()
            .set(&DataKey::AttestationRegistry, &attestation_registry);
        env.storage().instance().set(&DataKey::TotalDeposited, &0i128);
        env.storage().instance().set(&DataKey::TotalAvailable, &0i128);
        env.storage()
            .instance()
            .set(&DataKey::TotalDeployedPrincipal, &0i128);
        env.storage().instance().set(&DataKey::MinDeposit, &1i128);
        env.storage()
            .instance()
            .set(&DataKey::WithdrawalDuration, &172_800u64); // 2 days
        env.storage()
            .instance()
            .set(&DataKey::WithdrawalGap, &1_200u64); // 20 minutes
        env.storage().instance().set(&DataKey::Initialized, &true);
        Ok(())
    }

    pub fn set_investor_status(env: Env, investor: Address, approved: bool) -> Result<(), Error> {
        let admin: Address = Self::role(&env, &DataKey::Admin)?;
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::InvestorStatus(investor.clone()), &approved);
        InvestorStatusSet { investor, approved }.publish(&env);
        Ok(())
    }

    pub fn deposit(
        env: Env,
        investor: Address,
        amount: i128,
        operation_id: BytesN<32>,
    ) -> Result<(), Error> {
        Self::ensure_init(&env)?;
        investor.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let approved: bool = env
            .storage()
            .persistent()
            .get(&DataKey::InvestorStatus(investor.clone()))
            .unwrap_or(false);
        if !approved {
            return Err(Error::InvestorNotApproved);
        }
        Self::consume_op(&env, &operation_id)?;

        let asset: Address = Self::role(&env, &DataKey::SettlementAsset)?;
        let token = token::Client::new(&env, &asset);
        // Transfer first — units only credited if transfer succeeds (panic otherwise).
        token.transfer(&investor, &env.current_contract_address(), &amount);

        let units: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::InvestorUnits(investor.clone()))
            .unwrap_or(0);
        // 1:1 units for Track A (NAV share pricing is Track B extension via share_price).
        let new_units = units + amount;
        env.storage()
            .persistent()
            .set(&DataKey::InvestorUnits(investor.clone()), &new_units);

        let deposited: i128 = env.storage().instance().get(&DataKey::TotalDeposited).unwrap();
        let available: i128 = env.storage().instance().get(&DataKey::TotalAvailable).unwrap();
        env.storage()
            .instance()
            .set(&DataKey::TotalDeposited, &(deposited + amount));
        env.storage()
            .instance()
            .set(&DataKey::TotalAvailable, &(available + amount));

        Deposited {
            investor,
            amount,
            operation_id,
            units: new_units,
        }
        .publish(&env);
        Ok(())
    }

    pub fn register_receivable(
        env: Env,
        receivable_id: String,
        merchant_id: String,
        merchant_payout_address: Address,
        advance_amount: i128,
        contractual_repayment_amount: i128,
        maturity_timestamp: u64,
    ) -> Result<(), Error> {
        let originator: Address = Self::role(&env, &DataKey::Originator)?;
        originator.require_auth();

        if env
            .storage()
            .persistent()
            .has(&DataKey::Receivable(receivable_id.clone()))
        {
            return Err(Error::ReceivableExists);
        }
        if advance_amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if contractual_repayment_amount < advance_amount {
            return Err(Error::InvalidEconomics);
        }
        if maturity_timestamp <= env.ledger().timestamp() {
            return Err(Error::MaturityNotFuture);
        }

        let rec = Receivable {
            receivable_id: receivable_id.clone(),
            merchant_id: merchant_id.clone(),
            merchant_payout_address,
            advance_amount,
            contractual_repayment_amount,
            principal_outstanding: 0,
            total_repaid: 0,
            realized_income: 0,
            maturity_timestamp,
            status: ReceivableStatus::Registered,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Receivable(receivable_id.clone()), &rec);

        ReceivableRegistered {
            receivable_id,
            merchant_id,
            advance_amount,
            contractual_repayment_amount,
        }
        .publish(&env);
        Ok(())
    }

    pub fn approve_receivable(env: Env, receivable_id: String) -> Result<(), Error> {
        let risk: Address = Self::role(&env, &DataKey::RiskManager)?;
        risk.require_auth();
        let mut rec: Receivable = env
            .storage()
            .persistent()
            .get(&DataKey::Receivable(receivable_id.clone()))
            .ok_or(Error::ReceivableNotFound)?;
        if rec.status != ReceivableStatus::Registered {
            return Err(Error::InvalidStatus);
        }
        rec.status = ReceivableStatus::Approved;
        env.storage()
            .persistent()
            .set(&DataKey::Receivable(receivable_id.clone()), &rec);
        ReceivableApproved { receivable_id }.publish(&env);
        Ok(())
    }

    pub fn fund_receivable(
        env: Env,
        receivable_id: String,
        operation_id: BytesN<32>,
    ) -> Result<(), Error> {
        let pool_manager: Address = Self::role(&env, &DataKey::PoolManager)?;
        pool_manager.require_auth();
        Self::consume_op(&env, &operation_id)?;

        let mut rec: Receivable = env
            .storage()
            .persistent()
            .get(&DataKey::Receivable(receivable_id.clone()))
            .ok_or(Error::ReceivableNotFound)?;
        if rec.status != ReceivableStatus::Approved {
            return Err(Error::InvalidStatus);
        }

        // Dual attestation gate
        let registry: Address = Self::role(&env, &DataKey::AttestationRegistry)?;
        let subject = Self::subject_from_merchant_id(&env, &rec.merchant_id);
        if !Self::attestation_valid(&env, &registry, &subject, 1)
            || !Self::attestation_valid(&env, &registry, &subject, 2)
        {
            return Err(Error::AttestationMissing);
        }

        let available: i128 = env.storage().instance().get(&DataKey::TotalAvailable).unwrap();
        if available < rec.advance_amount {
            return Err(Error::InsufficientLiquidity);
        }

        let asset: Address = Self::role(&env, &DataKey::SettlementAsset)?;
        let token = token::Client::new(&env, &asset);
        // Transfer advance to merchant. Missing trustline / insufficient auth fails the invoke.
        token.transfer(
            &env.current_contract_address(),
            &rec.merchant_payout_address,
            &rec.advance_amount,
        );

        let deployed: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalDeployedPrincipal)
            .unwrap();
        env.storage()
            .instance()
            .set(&DataKey::TotalAvailable, &(available - rec.advance_amount));
        env.storage().instance().set(
            &DataKey::TotalDeployedPrincipal,
            &(deployed + rec.advance_amount),
        );

        rec.principal_outstanding = rec.advance_amount;
        rec.status = ReceivableStatus::Funded;
        let amount = rec.advance_amount;
        let merchant = rec.merchant_payout_address.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Receivable(receivable_id.clone()), &rec);

        ReceivableFunded {
            receivable_id: receivable_id.clone(),
            amount,
            merchant: merchant.clone(),
            operation_id,
        }
        .publish(&env);
        ReceivableDisbursed {
            receivable_id,
            merchant,
            amount,
        }
        .publish(&env);
        Ok(())
    }

    pub fn repay(
        env: Env,
        receivable_id: String,
        payer: Address,
        amount: i128,
        operation_id: BytesN<32>,
    ) -> Result<(), Error> {
        payer.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        Self::consume_op(&env, &operation_id)?;

        let mut rec: Receivable = env
            .storage()
            .persistent()
            .get(&DataKey::Receivable(receivable_id.clone()))
            .ok_or(Error::ReceivableNotFound)?;
        if rec.status != ReceivableStatus::Funded && rec.status != ReceivableStatus::PartiallyRepaid
        {
            return Err(Error::InvalidStatus);
        }

        let remaining = rec.contractual_repayment_amount - rec.total_repaid;
        if amount > remaining {
            return Err(Error::OverRepayment);
        }

        let asset: Address = Self::role(&env, &DataKey::SettlementAsset)?;
        let token = token::Client::new(&env, &asset);
        token.transfer(&payer, &env.current_contract_address(), &amount);

        // Apply to principal first; excess is realized income.
        let to_principal = if amount <= rec.principal_outstanding {
            amount
        } else {
            rec.principal_outstanding
        };
        let income = amount - to_principal;
        rec.principal_outstanding -= to_principal;
        rec.realized_income += income;
        rec.total_repaid += amount;

        let available: i128 = env.storage().instance().get(&DataKey::TotalAvailable).unwrap();
        let deployed: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalDeployedPrincipal)
            .unwrap();
        env.storage()
            .instance()
            .set(&DataKey::TotalAvailable, &(available + amount));
        env.storage().instance().set(
            &DataKey::TotalDeployedPrincipal,
            &(deployed - to_principal).max(0),
        );

        if rec.total_repaid == rec.contractual_repayment_amount {
            rec.status = ReceivableStatus::Settled;
        } else {
            rec.status = ReceivableStatus::PartiallyRepaid;
        }
        let status = rec.status.clone();
        let total_repaid = rec.total_repaid;
        env.storage()
            .persistent()
            .set(&DataKey::Receivable(receivable_id.clone()), &rec);

        Repaid {
            receivable_id,
            payer,
            amount,
            total_repaid,
            status,
            operation_id,
        }
        .publish(&env);
        Ok(())
    }

    pub fn get_receivable(env: Env, receivable_id: String) -> Option<Receivable> {
        env.storage()
            .persistent()
            .get(&DataKey::Receivable(receivable_id))
    }

    pub fn get_investor_units(env: Env, investor: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::InvestorUnits(investor))
            .unwrap_or(0)
    }

    pub fn get_pool_accounting(env: Env) -> PoolAccounting {
        PoolAccounting {
            total_deposited: env
                .storage()
                .instance()
                .get(&DataKey::TotalDeposited)
                .unwrap_or(0),
            total_available: env
                .storage()
                .instance()
                .get(&DataKey::TotalAvailable)
                .unwrap_or(0),
            total_deployed_principal: env
                .storage()
                .instance()
                .get(&DataKey::TotalDeployedPrincipal)
                .unwrap_or(0),
        }
    }

    // ── Track B: simple redeem (investor redemption) ──────────────────────
    pub fn redeem(env: Env, investor: Address, units: i128, operation_id: BytesN<32>) -> Result<(), Error> {
        investor.require_auth();
        if units <= 0 {
            return Err(Error::InvalidAmount);
        }
        Self::consume_op(&env, &operation_id)?;
        let held: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::InvestorUnits(investor.clone()))
            .unwrap_or(0);
        if units > held {
            return Err(Error::InvalidAmount);
        }
        let available: i128 = env.storage().instance().get(&DataKey::TotalAvailable).unwrap_or(0);
        if units > available {
            return Err(Error::InsufficientLiquidity);
        }
        let asset: Address = Self::role(&env, &DataKey::SettlementAsset)?;
        let token = token::Client::new(&env, &asset);
        token.transfer(&env.current_contract_address(), &investor, &units);
        env.storage()
            .persistent()
            .set(&DataKey::InvestorUnits(investor), &(held - units));
        env.storage()
            .instance()
            .set(&DataKey::TotalAvailable, &(available - units));
        Ok(())
    }
}

impl TradeCreditPool {
    fn ensure_init(env: &Env) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::NotInitialized);
        }
        Ok(())
    }

    fn role(env: &Env, key: &DataKey) -> Result<Address, Error> {
        Self::ensure_init(env)?;
        env.storage()
            .instance()
            .get(key)
            .ok_or(Error::NotInitialized)
    }

    fn consume_op(env: &Env, operation_id: &BytesN<32>) -> Result<(), Error> {
        let key = DataKey::UsedOperationId(operation_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::OperationIdReused);
        }
        env.storage().persistent().set(&key, &true);
        Ok(())
    }

    fn subject_from_merchant_id(env: &Env, merchant_id: &String) -> BytesN<32> {
        // Opaque subject: SHA256 over the Soroban String payload (Bytes::from).
        let bytes = soroban_sdk::Bytes::from(merchant_id.clone());
        let hash = env.crypto().sha256(&bytes);
        BytesN::from_array(env, &hash.to_array())
    }

    fn attestation_valid(env: &Env, registry: &Address, subject: &BytesN<32>, ty: u32) -> bool {
        let client = AttestationRegistryClient::new(env, registry);
        let at = if ty == 1 {
            AttestationType::MerchantApproved
        } else {
            AttestationType::DocumentsVerified
        };
        client.is_attestation_valid(subject, &at)
    }
}

#[cfg(test)]
mod test;
