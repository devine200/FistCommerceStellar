#![no_std]
//! Fist Commerce AttestationRegistry
//!
//! Records opaque outcomes of off-chain checks. Does not store PII
//! (merchant names, invoices, KYC docs, insurance, bank details, personal data).

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
    IssuerNotAuthorized = 4,
    NotFound = 5,
    StaleRevision = 6,
    InvalidInput = 7,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationType {
    MerchantApproved = 1,
    DocumentsVerified = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationStatus {
    Active = 1,
    Revoked = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    pub subject_id: BytesN<32>,
    pub attestation_type: AttestationType,
    pub issuer: Address,
    pub evidence_commitment: BytesN<32>,
    pub issued_at: u64,
    pub expires_at: u64,
    pub revision: u32,
    pub status: AttestationStatus,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Initialized,
    Issuer(AttestationType, Address),
    Revoker(Address),
    Record(BytesN<32>, AttestationType),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuerAuthorized {
    #[topic]
    pub attestation_type: AttestationType,
    #[topic]
    pub issuer: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuerRemoved {
    #[topic]
    pub attestation_type: AttestationType,
    #[topic]
    pub issuer: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationIssued {
    #[topic]
    pub subject_id: BytesN<32>,
    #[topic]
    pub attestation_type: AttestationType,
    pub issuer: Address,
    pub evidence_commitment: BytesN<32>,
    pub expires_at: u64,
    pub revision: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationRevoked {
    #[topic]
    pub subject_id: BytesN<32>,
    #[topic]
    pub attestation_type: AttestationType,
    pub revision: u32,
    pub by: Address,
}

#[contract]
pub struct AttestationRegistry;

#[contractimpl]
impl AttestationRegistry {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);
        Ok(())
    }

    pub fn authorize_issuer(
        env: Env,
        attestation_type: AttestationType,
        issuer: Address,
    ) -> Result<(), Error> {
        let admin = Self::require_admin(&env)?;
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::Issuer(attestation_type.clone(), issuer.clone()), &true);
        IssuerAuthorized {
            attestation_type,
            issuer,
        }
        .publish(&env);
        Ok(())
    }

    pub fn remove_issuer(
        env: Env,
        attestation_type: AttestationType,
        issuer: Address,
    ) -> Result<(), Error> {
        let admin = Self::require_admin(&env)?;
        admin.require_auth();
        env.storage()
            .persistent()
            .remove(&DataKey::Issuer(attestation_type.clone(), issuer.clone()));
        IssuerRemoved {
            attestation_type,
            issuer,
        }
        .publish(&env);
        Ok(())
    }

    /// Admin may grant an explicit revocation role.
    pub fn set_revoker(env: Env, revoker: Address, allowed: bool) -> Result<(), Error> {
        let admin = Self::require_admin(&env)?;
        admin.require_auth();
        if allowed {
            env.storage()
                .persistent()
                .set(&DataKey::Revoker(revoker), &true);
        } else {
            env.storage()
                .persistent()
                .remove(&DataKey::Revoker(revoker));
        }
        Ok(())
    }

    /// Issue attestation. `issuer` must be authorized for the type and must authorize.
    pub fn issue_attestation(
        env: Env,
        issuer: Address,
        subject_id: BytesN<32>,
        attestation_type: AttestationType,
        evidence_commitment: BytesN<32>,
        expires_at: u64,
    ) -> Result<(), Error> {
        Self::ensure_initialized(&env)?;
        issuer.require_auth();
        if !env
            .storage()
            .persistent()
            .has(&DataKey::Issuer(attestation_type.clone(), issuer.clone()))
        {
            return Err(Error::IssuerNotAuthorized);
        }

        let now = env.ledger().timestamp();
        if expires_at <= now {
            return Err(Error::InvalidInput);
        }

        let key = DataKey::Record(subject_id.clone(), attestation_type.clone());
        let revision = if let Some(existing) = env.storage().persistent().get::<_, Attestation>(&key)
        {
            existing.revision.saturating_add(1)
        } else {
            1
        };

        let record = Attestation {
            subject_id: subject_id.clone(),
            attestation_type: attestation_type.clone(),
            issuer: issuer.clone(),
            evidence_commitment: evidence_commitment.clone(),
            issued_at: now,
            expires_at,
            revision,
            status: AttestationStatus::Active,
        };
        env.storage().persistent().set(&key, &record);

        AttestationIssued {
            subject_id,
            attestation_type,
            issuer,
            evidence_commitment,
            expires_at,
            revision,
        }
        .publish(&env);
        Ok(())
    }

    /// Revoke. `by` must be original issuer, revoker role, or admin. Older revision cannot overwrite newer.
    pub fn revoke_attestation(
        env: Env,
        by: Address,
        subject_id: BytesN<32>,
        attestation_type: AttestationType,
        revision: u32,
    ) -> Result<(), Error> {
        Self::ensure_initialized(&env)?;
        by.require_auth();

        let key = DataKey::Record(subject_id.clone(), attestation_type.clone());
        let mut record: Attestation = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NotFound)?;

        // An older revision must not overwrite a newer attestation.
        if revision < record.revision {
            return Err(Error::StaleRevision);
        }
        if revision > record.revision {
            return Err(Error::StaleRevision);
        }

        let admin = Self::require_admin(&env)?;
        let allowed = by == record.issuer
            || by == admin
            || env.storage().persistent().has(&DataKey::Revoker(by.clone()));
        if !allowed {
            return Err(Error::Unauthorized);
        }

        record.status = AttestationStatus::Revoked;
        env.storage().persistent().set(&key, &record);

        AttestationRevoked {
            subject_id,
            attestation_type,
            revision,
            by,
        }
        .publish(&env);
        Ok(())
    }

    pub fn is_attestation_valid(
        env: Env,
        subject_id: BytesN<32>,
        attestation_type: AttestationType,
    ) -> bool {
        let key = DataKey::Record(subject_id, attestation_type);
        let Some(record) = env.storage().persistent().get::<_, Attestation>(&key) else {
            return false;
        };
        if record.status != AttestationStatus::Active {
            return false;
        }
        env.ledger().timestamp() < record.expires_at
    }

    pub fn get_attestation(
        env: Env,
        subject_id: BytesN<32>,
        attestation_type: AttestationType,
    ) -> Option<Attestation> {
        env.storage()
            .persistent()
            .get(&DataKey::Record(subject_id, attestation_type))
    }

    pub fn admin(env: Env) -> Result<Address, Error> {
        Self::require_admin(&env)
    }
}

impl AttestationRegistry {
    fn ensure_initialized(env: &Env) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::NotInitialized);
        }
        Ok(())
    }

    fn require_admin(env: &Env) -> Result<Address, Error> {
        Self::ensure_initialized(env)?;
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)
    }
}

#[cfg(test)]
mod test;
