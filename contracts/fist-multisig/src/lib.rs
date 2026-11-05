#![no_std]
//! Fist Commerce FistMultisig — Track B threshold admin (Soroban analogue of FistMultisigAccount).
//! Owners must each `require_auth` when confirming; execution requires threshold confirmations.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, BytesN, Env, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    AlreadyConfirmed = 4,
    NotFound = 5,
    ThresholdNotMet = 6,
    InvalidThreshold = 7,
    AlreadyExecuted = 8,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub action_hash: BytesN<32>,
    pub confirmations: u32,
    pub executed: bool,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Initialized,
    Threshold,
    OwnerCount,
    IsOwner(Address),
    ProposalCount,
    Proposal(u64),
    Confirmed(u64, Address),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalSubmitted {
    #[topic]
    pub id: u64,
    pub action_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalConfirmed {
    #[topic]
    pub id: u64,
    pub owner: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalExecuted {
    #[topic]
    pub id: u64,
}

#[contract]
pub struct FistMultisig;

#[contractimpl]
impl FistMultisig {
    pub fn initialize(env: Env, owners: Vec<Address>, threshold: u32) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }
        let n = owners.len();
        if threshold == 0 || threshold > n {
            return Err(Error::InvalidThreshold);
        }
        for i in 0..n {
            let o = owners.get(i).unwrap();
            o.require_auth();
            env.storage()
                .persistent()
                .set(&DataKey::IsOwner(o), &true);
        }
        env.storage().instance().set(&DataKey::Threshold, &threshold);
        env.storage().instance().set(&DataKey::OwnerCount, &n);
        env.storage().instance().set(&DataKey::ProposalCount, &0u64);
        env.storage().instance().set(&DataKey::Initialized, &true);
        Ok(())
    }

    pub fn submit_proposal(env: Env, owner: Address, action_hash: BytesN<32>) -> Result<u64, Error> {
        owner.require_auth();
        Self::require_owner(&env, &owner)?;
        let mut count: u64 = env.storage().instance().get(&DataKey::ProposalCount).unwrap();
        let id = count;
        count += 1;
        env.storage().instance().set(&DataKey::ProposalCount, &count);
        let p = Proposal {
            id,
            action_hash: action_hash.clone(),
            confirmations: 0,
            executed: false,
        };
        env.storage().persistent().set(&DataKey::Proposal(id), &p);
        ProposalSubmitted { id, action_hash }.publish(&env);
        Ok(id)
    }

    pub fn confirm(env: Env, owner: Address, proposal_id: u64) -> Result<(), Error> {
        owner.require_auth();
        Self::require_owner(&env, &owner)?;
        let mut p: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::NotFound)?;
        if p.executed {
            return Err(Error::AlreadyExecuted);
        }
        let ckey = DataKey::Confirmed(proposal_id, owner.clone());
        if env.storage().persistent().has(&ckey) {
            return Err(Error::AlreadyConfirmed);
        }
        env.storage().persistent().set(&ckey, &true);
        p.confirmations += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &p);
        ProposalConfirmed {
            id: proposal_id,
            owner,
        }
        .publish(&env);
        Ok(())
    }

    pub fn execute(env: Env, proposal_id: u64) -> Result<(), Error> {
        let mut p: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::NotFound)?;
        if p.executed {
            return Err(Error::AlreadyExecuted);
        }
        let threshold: u32 = env.storage().instance().get(&DataKey::Threshold).unwrap();
        if p.confirmations < threshold {
            return Err(Error::ThresholdNotMet);
        }
        p.executed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &p);
        ProposalExecuted { id: proposal_id }.publish(&env);
        Ok(())
    }

    pub fn get_proposal(env: Env, proposal_id: u64) -> Option<Proposal> {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
    }

    pub fn threshold(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::Threshold)
            .unwrap_or(0)
    }

    pub fn is_owner(env: Env, addr: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::IsOwner(addr))
            .unwrap_or(false)
    }
}

impl FistMultisig {
    fn require_owner(env: &Env, owner: &Address) -> Result<(), Error> {
        if !env
            .storage()
            .persistent()
            .get::<_, bool>(&DataKey::IsOwner(owner.clone()))
            .unwrap_or(false)
        {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, vec};

    #[test]
    fn threshold_flow() {
        let env = Env::default();
        env.mock_all_auths();
        let a = Address::generate(&env);
        let b = Address::generate(&env);
        let c = Address::generate(&env);
        let id = env.register(FistMultisig, ());
        let client = FistMultisigClient::new(&env, &id);
        let owners = vec![&env, a.clone(), b.clone(), c.clone()];
        client.initialize(&owners, &2);
        let hash = BytesN::from_array(&env, &[1u8; 32]);
        let pid = client.submit_proposal(&a, &hash);
        client.confirm(&a, &pid);
        assert!(client.try_execute(&pid).is_err());
        client.confirm(&b, &pid);
        client.execute(&pid);
        assert!(client.get_proposal(&pid).unwrap().executed);
    }
}
