#![no_std]
//! Fist Commerce ProtocolController — Track B circuit breakers (mirrors EVM ProtocolController).

use soroban_sdk::{contract, contracterror, contractevent, contractimpl, contracttype, Address, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Initialized,
    Admin,
    Guardian,
    Paused,
    DepositsPaused,
    WithdrawalsPaused,
    FundingPaused,
    RepaymentsPaused,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PausedSet {
    pub paused: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositsPausedSet {
    pub paused: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawalsPausedSet {
    pub paused: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundingPausedSet {
    pub paused: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepaymentsPausedSet {
    pub paused: bool,
}

#[contract]
pub struct ProtocolController;

#[contractimpl]
impl ProtocolController {
    pub fn initialize(env: Env, admin: Address, guardian: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Guardian, &guardian);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage().instance().set(&DataKey::DepositsPaused, &false);
        env.storage().instance().set(&DataKey::WithdrawalsPaused, &false);
        env.storage().instance().set(&DataKey::FundingPaused, &false);
        env.storage().instance().set(&DataKey::RepaymentsPaused, &false);
        env.storage().instance().set(&DataKey::Initialized, &true);
        Ok(())
    }

    pub fn set_paused(env: Env, paused: bool) -> Result<(), Error> {
        Self::require_guardian(&env)?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        PausedSet { paused }.publish(&env);
        Ok(())
    }

    pub fn set_deposits_paused(env: Env, paused: bool) -> Result<(), Error> {
        Self::require_guardian(&env)?;
        env.storage().instance().set(&DataKey::DepositsPaused, &paused);
        DepositsPausedSet { paused }.publish(&env);
        Ok(())
    }

    pub fn set_withdrawals_paused(env: Env, paused: bool) -> Result<(), Error> {
        Self::require_guardian(&env)?;
        env.storage().instance().set(&DataKey::WithdrawalsPaused, &paused);
        WithdrawalsPausedSet { paused }.publish(&env);
        Ok(())
    }

    pub fn set_funding_paused(env: Env, paused: bool) -> Result<(), Error> {
        Self::require_guardian(&env)?;
        env.storage().instance().set(&DataKey::FundingPaused, &paused);
        FundingPausedSet { paused }.publish(&env);
        Ok(())
    }

    pub fn set_repayments_paused(env: Env, paused: bool) -> Result<(), Error> {
        Self::require_guardian(&env)?;
        env.storage().instance().set(&DataKey::RepaymentsPaused, &paused);
        RepaymentsPausedSet { paused }.publish(&env);
        Ok(())
    }

    pub fn is_system_active(env: Env) -> bool {
        let paused: bool = env.storage().instance().get(&DataKey::Paused).unwrap_or(true);
        !paused
    }

    pub fn deposits_allowed(env: Env) -> bool {
        Self::is_system_active(env.clone())
            && !env
                .storage()
                .instance()
                .get::<_, bool>(&DataKey::DepositsPaused)
                .unwrap_or(true)
    }

    pub fn withdrawals_allowed(env: Env) -> bool {
        Self::is_system_active(env.clone())
            && !env
                .storage()
                .instance()
                .get::<_, bool>(&DataKey::WithdrawalsPaused)
                .unwrap_or(true)
    }

    pub fn funding_allowed(env: Env) -> bool {
        Self::is_system_active(env.clone())
            && !env
                .storage()
                .instance()
                .get::<_, bool>(&DataKey::FundingPaused)
                .unwrap_or(true)
    }

    pub fn repayments_allowed(env: Env) -> bool {
        Self::is_system_active(env.clone())
            && !env
                .storage()
                .instance()
                .get::<_, bool>(&DataKey::RepaymentsPaused)
                .unwrap_or(true)
    }
}

impl ProtocolController {
    fn require_guardian(env: &Env) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::NotInitialized);
        }
        let guardian: Address = env
            .storage()
            .instance()
            .get(&DataKey::Guardian)
            .ok_or(Error::NotInitialized)?;
        guardian.require_auth();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn pause_flags() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let guardian = Address::generate(&env);
        let id = env.register(ProtocolController, ());
        let c = ProtocolControllerClient::new(&env, &id);
        c.initialize(&admin, &guardian);
        assert!(c.is_system_active());
        c.set_paused(&true);
        assert!(!c.is_system_active());
        c.set_paused(&false);
        c.set_funding_paused(&true);
        assert!(!c.funding_allowed());
    }
}
