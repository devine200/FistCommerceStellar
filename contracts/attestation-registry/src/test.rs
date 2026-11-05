#![cfg(test)]
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env,
};

fn setup() -> (Env, Address, Address, AttestationRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(AttestationRegistry, ());
    let client = AttestationRegistryClient::new(&env, &contract_id);
    client.initialize(&admin);
    (env, admin, contract_id, client)
}

fn subject(env: &Env, n: u8) -> BytesN<32> {
    let mut b = [0u8; 32];
    b[31] = n;
    BytesN::from_array(env, &b)
}

fn commitment(env: &Env, n: u8) -> BytesN<32> {
    let mut b = [0u8; 32];
    b[0] = n;
    BytesN::from_array(env, &b)
}

#[test]
fn admin_can_authorize_issuer() {
    let (env, _admin, _, client) = setup();
    let issuer = Address::generate(&env);
    client.authorize_issuer(&AttestationType::MerchantApproved, &issuer);
    assert!(client.try_authorize_issuer(&AttestationType::MerchantApproved, &issuer).is_ok());
}

#[test]
fn non_admin_cannot_authorize_issuer() {
    use soroban_sdk::{testutils::MockAuth, testutils::MockAuthInvoke, IntoVal};

    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(AttestationRegistry, ());
    let client = AttestationRegistryClient::new(&env, &contract_id);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (admin.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin);

    let issuer = Address::generate(&env);
    // No admin auth mocked for authorize_issuer — must fail host auth
    let res = client.try_authorize_issuer(&AttestationType::DocumentsVerified, &issuer);
    assert!(res.is_err());
}

#[test]
fn authorized_issuer_can_issue() {
    let (env, _, _, client) = setup();
    let issuer = Address::generate(&env);
    client.authorize_issuer(&AttestationType::MerchantApproved, &issuer);
    let sub = subject(&env, 1);
    let exp = env.ledger().timestamp() + 86_400;
    client.issue_attestation(
        &issuer,
        &sub,
        &AttestationType::MerchantApproved,
        &commitment(&env, 9),
        &exp,
    );
    assert!(client.is_attestation_valid(&sub, &AttestationType::MerchantApproved));
}

#[test]
fn unauthorized_issuer_rejected() {
    let (env, _, _, client) = setup();
    let issuer = Address::generate(&env);
    let sub = subject(&env, 2);
    let exp = env.ledger().timestamp() + 86_400;
    let res = client.try_issue_attestation(
        &issuer,
        &sub,
        &AttestationType::MerchantApproved,
        &commitment(&env, 1),
        &exp,
    );
    assert!(res.is_err());
}

#[test]
fn issuer_cannot_issue_unassigned_type() {
    let (env, _, _, client) = setup();
    let issuer = Address::generate(&env);
    client.authorize_issuer(&AttestationType::MerchantApproved, &issuer);
    let sub = subject(&env, 3);
    let exp = env.ledger().timestamp() + 86_400;
    let res = client.try_issue_attestation(
        &issuer,
        &sub,
        &AttestationType::DocumentsVerified,
        &commitment(&env, 2),
        &exp,
    );
    assert!(res.is_err());
}

#[test]
fn revoked_attestation_is_invalid() {
    let (env, _, _, client) = setup();
    let issuer = Address::generate(&env);
    client.authorize_issuer(&AttestationType::DocumentsVerified, &issuer);
    let sub = subject(&env, 4);
    let exp = env.ledger().timestamp() + 86_400;
    client.issue_attestation(
        &issuer,
        &sub,
        &AttestationType::DocumentsVerified,
        &commitment(&env, 3),
        &exp,
    );
    client.revoke_attestation(&issuer, &sub, &AttestationType::DocumentsVerified, &1);
    assert!(!client.is_attestation_valid(&sub, &AttestationType::DocumentsVerified));
}

#[test]
fn expired_attestation_is_invalid() {
    let (env, _, _, client) = setup();
    let issuer = Address::generate(&env);
    client.authorize_issuer(&AttestationType::MerchantApproved, &issuer);
    let sub = subject(&env, 5);
    let exp = env.ledger().timestamp() + 10;
    client.issue_attestation(
        &issuer,
        &sub,
        &AttestationType::MerchantApproved,
        &commitment(&env, 4),
        &exp,
    );
    // Advance ledger past expiry
    env.ledger().set_timestamp(exp + 1);
    assert!(!client.is_attestation_valid(&sub, &AttestationType::MerchantApproved));
}

#[test]
fn newer_revision_supersedes_older() {
    let (env, _, _, client) = setup();
    let issuer = Address::generate(&env);
    client.authorize_issuer(&AttestationType::MerchantApproved, &issuer);
    let sub = subject(&env, 6);
    let exp = env.ledger().timestamp() + 86_400;
    client.issue_attestation(
        &issuer,
        &sub,
        &AttestationType::MerchantApproved,
        &commitment(&env, 1),
        &exp,
    );
    client.issue_attestation(
        &issuer,
        &sub,
        &AttestationType::MerchantApproved,
        &commitment(&env, 2),
        &exp,
    );
    let rec = client.get_attestation(&sub, &AttestationType::MerchantApproved).unwrap();
    assert_eq!(rec.revision, 2);
    assert_eq!(rec.evidence_commitment, commitment(&env, 2));
    // Revoking old revision fails
    let res = client.try_revoke_attestation(&issuer, &sub, &AttestationType::MerchantApproved, &1);
    assert!(res.is_err());
}

#[test]
fn active_unexpired_attestation_is_valid() {
    let (env, _, _, client) = setup();
    let issuer = Address::generate(&env);
    client.authorize_issuer(&AttestationType::DocumentsVerified, &issuer);
    let sub = subject(&env, 7);
    let exp = env.ledger().timestamp() + 86_400;
    client.issue_attestation(
        &issuer,
        &sub,
        &AttestationType::DocumentsVerified,
        &commitment(&env, 7),
        &exp,
    );
    assert!(client.is_attestation_valid(&sub, &AttestationType::DocumentsVerified));
}
