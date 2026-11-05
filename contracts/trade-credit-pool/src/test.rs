#![cfg(test)]
use super::*;
use attestation_registry::{AttestationRegistry, AttestationRegistryClient, AttestationType};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, BytesN, Env, String,
};

fn op(env: &Env, n: u8) -> BytesN<32> {
    let mut b = [0u8; 32];
    b[31] = n;
    BytesN::from_array(env, &b)
}

fn commitment(env: &Env, n: u8) -> BytesN<32> {
    let mut b = [0u8; 32];
    b[0] = n;
    BytesN::from_array(env, &b)
}

struct Ctx {
    env: Env,
    admin: Address,
    pool_manager: Address,
    originator: Address,
    risk_manager: Address,
    investor: Address,
    merchant: Address,
    issuer: Address,
    token: Address,
    registry: Address,
    pool: Address,
    registry_client: AttestationRegistryClient<'static>,
    pool_client: TradeCreditPoolClient<'static>,
    token_admin: StellarAssetClient<'static>,
    token_client: TokenClient<'static>,
}

fn setup() -> Ctx {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let pool_manager = Address::generate(&env);
    let originator = Address::generate(&env);
    let risk_manager = Address::generate(&env);
    let investor = Address::generate(&env);
    let merchant = Address::generate(&env);
    let issuer = Address::generate(&env);

    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token = sac.address();
    let token_admin = StellarAssetClient::new(&env, &token);
    let token_client = TokenClient::new(&env, &token);

    let registry = env.register(AttestationRegistry, ());
    let registry_client = AttestationRegistryClient::new(&env, &registry);
    registry_client.initialize(&admin);
    registry_client.authorize_issuer(&AttestationType::MerchantApproved, &issuer);
    registry_client.authorize_issuer(&AttestationType::DocumentsVerified, &issuer);

    let pool = env.register(TradeCreditPool, ());
    let pool_client = TradeCreditPoolClient::new(&env, &pool);
    pool_client.initialize(
        &admin,
        &pool_manager,
        &originator,
        &risk_manager,
        &token,
        &registry,
    );

    Ctx {
        env,
        admin,
        pool_manager,
        originator,
        risk_manager,
        investor,
        merchant,
        issuer,
        token,
        registry,
        pool,
        registry_client,
        pool_client,
        token_admin,
        token_client,
    }
}

fn subject_for(env: &Env, merchant_id: &str) -> BytesN<32> {
    let s = String::from_str(env, merchant_id);
    let hash = env.crypto().sha256(&s.into());
    BytesN::from_array(env, &hash.to_array())
}

fn issue_both(ctx: &Ctx, merchant_id: &str) {
    let sub = subject_for(&ctx.env, merchant_id);
    let exp = ctx.env.ledger().timestamp() + 86_400;
    ctx.registry_client.issue_attestation(
        &ctx.issuer,
        &sub,
        &AttestationType::MerchantApproved,
        &commitment(&ctx.env, 1),
        &exp,
    );
    ctx.registry_client.issue_attestation(
        &ctx.issuer,
        &sub,
        &AttestationType::DocumentsVerified,
        &commitment(&ctx.env, 2),
        &exp,
    );
}

#[test]
fn admin_can_approve_investor() {
    let ctx = setup();
    ctx.pool_client
        .set_investor_status(&ctx.investor, &true);
    // deposit path checks approval
    ctx.token_admin.mint(&ctx.investor, &10_000);
    ctx.pool_client
        .deposit(&ctx.investor, &1_000, &op(&ctx.env, 1));
    assert_eq!(ctx.pool_client.get_investor_units(&ctx.investor), 1_000);
}

#[test]
fn non_approved_investor_cannot_deposit() {
    let ctx = setup();
    ctx.token_admin.mint(&ctx.investor, &10_000);
    let res = ctx
        .pool_client
        .try_deposit(&ctx.investor, &1_000, &op(&ctx.env, 1));
    assert!(res.is_err());
}

#[test]
fn deposit_credits_units_and_rejects_reused_op_id() {
    let ctx = setup();
    ctx.pool_client
        .set_investor_status(&ctx.investor, &true);
    ctx.token_admin.mint(&ctx.investor, &50_000);
    ctx.pool_client
        .deposit(&ctx.investor, &5_000, &op(&ctx.env, 2));
    assert_eq!(ctx.pool_client.get_investor_units(&ctx.investor), 5_000);
    let acct = ctx.pool_client.get_pool_accounting();
    assert_eq!(acct.total_deposited, 5_000);
    assert_eq!(acct.total_available, 5_000);
    let reuse = ctx
        .pool_client
        .try_deposit(&ctx.investor, &1_000, &op(&ctx.env, 2));
    assert!(reuse.is_err());
}

#[test]
fn duplicate_receivable_id_rejected() {
    let ctx = setup();
    let rid = String::from_str(&ctx.env, "FIST-DEMO-001");
    let mid = String::from_str(&ctx.env, "MERCHANT-DEMO-001");
    let mat = ctx.env.ledger().timestamp() + 86_400;
    ctx.pool_client.register_receivable(
        &rid,
        &mid,
        &ctx.merchant,
        &10_000,
        &10_500,
        &mat,
    );
    let res = ctx.pool_client.try_register_receivable(
        &rid,
        &mid,
        &ctx.merchant,
        &10_000,
        &10_500,
        &mat,
    );
    assert!(res.is_err());
}

#[test]
fn invalid_economics_rejected() {
    let ctx = setup();
    let rid = String::from_str(&ctx.env, "BAD-001");
    let mid = String::from_str(&ctx.env, "M-1");
    let mat = ctx.env.ledger().timestamp() + 86_400;
    let res = ctx.pool_client.try_register_receivable(
        &rid,
        &mid,
        &ctx.merchant,
        &10_000,
        &9_000,
        &mat,
    );
    assert!(res.is_err());
}

#[test]
fn funding_without_attestations_fails() {
    let ctx = setup();
    ctx.pool_client
        .set_investor_status(&ctx.investor, &true);
    ctx.token_admin.mint(&ctx.investor, &20_000);
    ctx.pool_client
        .deposit(&ctx.investor, &20_000, &op(&ctx.env, 10));

    let rid = String::from_str(&ctx.env, "FIST-DEMO-001");
    let mid = String::from_str(&ctx.env, "MERCHANT-DEMO-001");
    let mat = ctx.env.ledger().timestamp() + 86_400;
    ctx.pool_client.register_receivable(
        &rid,
        &mid,
        &ctx.merchant,
        &10_000,
        &10_500,
        &mat,
    );
    ctx.pool_client.approve_receivable(&rid);
    let res = ctx.pool_client.try_fund_receivable(&rid, &op(&ctx.env, 11));
    assert!(res.is_err());
}

#[test]
fn funding_succeeds_after_both_attestations_and_repay_settles() {
    let ctx = setup();
    ctx.pool_client
        .set_investor_status(&ctx.investor, &true);
    ctx.token_admin.mint(&ctx.investor, &20_000);
    ctx.pool_client
        .deposit(&ctx.investor, &20_000, &op(&ctx.env, 20));

    let rid = String::from_str(&ctx.env, "FIST-DEMO-001");
    let mid_str = "MERCHANT-DEMO-001";
    let mid = String::from_str(&ctx.env, mid_str);
    let mat = ctx.env.ledger().timestamp() + 86_400;
    ctx.pool_client.register_receivable(
        &rid,
        &mid,
        &ctx.merchant,
        &10_000,
        &10_500,
        &mat,
    );
    ctx.pool_client.approve_receivable(&rid);

    // Only merchant attestation — still fails
    let sub = subject_for(&ctx.env, mid_str);
    let exp = ctx.env.ledger().timestamp() + 86_400;
    ctx.registry_client.issue_attestation(
        &ctx.issuer,
        &sub,
        &AttestationType::MerchantApproved,
        &commitment(&ctx.env, 1),
        &exp,
    );
    assert!(ctx
        .pool_client
        .try_fund_receivable(&rid, &op(&ctx.env, 21))
        .is_err());

    ctx.registry_client.issue_attestation(
        &ctx.issuer,
        &sub,
        &AttestationType::DocumentsVerified,
        &commitment(&ctx.env, 2),
        &exp,
    );
    ctx.pool_client.fund_receivable(&rid, &op(&ctx.env, 22));

    assert_eq!(ctx.token_client.balance(&ctx.merchant), 10_000);
    let rec = ctx.pool_client.get_receivable(&rid).unwrap();
    assert_eq!(rec.status, ReceivableStatus::Funded);

    ctx.token_admin.mint(&ctx.merchant, &10_500);
    ctx.pool_client
        .repay(&rid, &ctx.merchant, &5_000, &op(&ctx.env, 23));
    let rec = ctx.pool_client.get_receivable(&rid).unwrap();
    assert_eq!(rec.status, ReceivableStatus::PartiallyRepaid);

    ctx.pool_client
        .repay(&rid, &ctx.merchant, &5_500, &op(&ctx.env, 24));
    let rec = ctx.pool_client.get_receivable(&rid).unwrap();
    assert_eq!(rec.status, ReceivableStatus::Settled);
    assert_eq!(rec.total_repaid, 10_500);
}
