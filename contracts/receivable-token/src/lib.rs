#![no_std]
//! Fist Commerce ReceivableToken — non-transferable loan-document token (not a settlement asset).
//! Documented technical reason for a custom token: bind opaque doc hashes to merchant financing,
//! separate from the FISTUSD Stellar Asset Contract used for settlement.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, BytesN, Env,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    NotFound = 4,
    TransferDisabled = 5,
    DocHashTaken = 6,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Initialized,
    Admin,
    Minter,
    TokenCount,
    Owner(u64),
    DocHash(u64),
    TokenByDoc(BytesN<32>),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Minted {
    #[topic]
    pub to: Address,
    #[topic]
    pub token_id: u64,
    pub doc_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Burned {
    #[topic]
    pub token_id: u64,
}

#[contract]
pub struct ReceivableToken;

#[contractimpl]
impl ReceivableToken {
    pub fn initialize(env: Env, admin: Address, minter: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Minter, &minter);
        env.storage().instance().set(&DataKey::TokenCount, &1u64);
        env.storage().instance().set(&DataKey::Initialized, &true);
        Ok(())
    }

    pub fn mint(env: Env, to: Address, doc_hash: BytesN<32>) -> Result<u64, Error> {
        let minter: Address = env
            .storage()
            .instance()
            .get(&DataKey::Minter)
            .ok_or(Error::NotInitialized)?;
        minter.require_auth();
        if env.storage().persistent().has(&DataKey::TokenByDoc(doc_hash.clone())) {
            return Err(Error::DocHashTaken);
        }
        let mut count: u64 = env.storage().instance().get(&DataKey::TokenCount).unwrap_or(1);
        let token_id = count;
        count += 1;
        env.storage().instance().set(&DataKey::TokenCount, &count);
        env.storage()
            .persistent()
            .set(&DataKey::Owner(token_id), &to);
        env.storage()
            .persistent()
            .set(&DataKey::DocHash(token_id), &doc_hash);
        env.storage()
            .persistent()
            .set(&DataKey::TokenByDoc(doc_hash.clone()), &token_id);
        Minted {
            to,
            token_id,
            doc_hash,
        }
        .publish(&env);
        Ok(token_id)
    }

    pub fn burn(env: Env, token_id: u64) -> Result<(), Error> {
        let minter: Address = env
            .storage()
            .instance()
            .get(&DataKey::Minter)
            .ok_or(Error::NotInitialized)?;
        minter.require_auth();
        let doc: BytesN<32> = env
            .storage()
            .persistent()
            .get(&DataKey::DocHash(token_id))
            .ok_or(Error::NotFound)?;
        env.storage().persistent().remove(&DataKey::Owner(token_id));
        env.storage().persistent().remove(&DataKey::DocHash(token_id));
        env.storage().persistent().remove(&DataKey::TokenByDoc(doc));
        Burned { token_id }.publish(&env);
        Ok(())
    }

    pub fn owner_of(env: Env, token_id: u64) -> Option<Address> {
        env.storage().persistent().get(&DataKey::Owner(token_id))
    }

    pub fn doc_hash(env: Env, token_id: u64) -> Option<BytesN<32>> {
        env.storage().persistent().get(&DataKey::DocHash(token_id))
    }

    pub fn owner_of_uri(env: Env, doc_hash: BytesN<32>) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::TokenByDoc(doc_hash))
    }

    /// Transfers are disabled except mint/burn edges (mirrors EVM ReceivableToken).
    pub fn transfer_from(_env: Env, _from: Address, _to: Address, _token_id: u64) -> Result<(), Error> {
        Err(Error::TransferDisabled)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn mint_burn_non_transfer() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let minter = Address::generate(&env);
        let user = Address::generate(&env);
        let id = env.register(ReceivableToken, ());
        let c = ReceivableTokenClient::new(&env, &id);
        c.initialize(&admin, &minter);
        let mut hash = [0u8; 32];
        hash[0] = 7;
        let doc = BytesN::from_array(&env, &hash);
        let tid = c.mint(&user, &doc);
        assert_eq!(c.owner_of(&tid), Some(user.clone()));
        assert!(c.try_transfer_from(&user, &admin, &tid).is_err());
        c.burn(&tid);
        assert!(c.owner_of(&tid).is_none());
    }
}
