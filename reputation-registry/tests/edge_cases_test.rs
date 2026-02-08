use identity_registry::{self, IdentityRegistry};
use multiversx_sc::types::{
    BigUint, EsdtLocalRole, ManagedAddress, ManagedBuffer, TokenIdentifier,
};
use multiversx_sc_scenario::imports::OptionalValue;
use multiversx_sc_scenario::rust_biguint;
use multiversx_sc_scenario::testing_framework::BlockchainStateWrapper;
use reputation_registry::*;
use validation_registry::{self, ValidationRegistry};

const ID_WASM_PATH: &str = "output/identity-registry.wasm";
const VAL_WASM_PATH: &str = "output/validation-registry.wasm";
const REP_WASM_PATH: &str = "output/reputation-registry.wasm";

#[test]
fn test_submit_feedback_failures() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let user_addr = b_mock.create_user_account(&rust_biguint!(0));
    let other_user_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));

    // Deploy All Registries
    let val_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        validation_registry::contract_obj,
        VAL_WASM_PATH,
    );
    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        ID_WASM_PATH,
    );
    let rep_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        reputation_registry::contract_obj,
        REP_WASM_PATH,
    );

    // Configure Reputation
    let val_addr = val_wrapper.address_ref().clone();
    let id_addr = id_wrapper.address_ref().clone();
    b_mock
        .execute_tx(&owner_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.set_validation_contract_address(ManagedAddress::from(val_addr));
            sc.set_identity_contract_address(ManagedAddress::from(id_addr));
        })
        .assert_ok();

    // Setup Agent
    b_mock
        .execute_tx(&owner_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.agent_token_id()
                .set_token_id(TokenIdentifier::from("AGENT-123456"));
        })
        .assert_ok();
    b_mock.set_esdt_local_roles(
        id_wrapper.address_ref(),
        b"AGENT-123456",
        &[EsdtLocalRole::NftCreate, EsdtLocalRole::NftUpdateAttributes],
    );
    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-01"),
                ManagedBuffer::from("uri"),
                ManagedBuffer::from("pk"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    // 1. Create Job (User)
    b_mock
        .execute_tx(&user_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.init_job(ManagedBuffer::from("job_1"), 1u64);
        })
        .assert_ok();

    // 2. Verify Job (Owner)
    b_mock
        .execute_tx(&owner_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.verify_job(ManagedBuffer::from("job_1"));
        })
        .assert_ok();

    // 3. Authorize Feedback (Agent)
    b_mock
        .execute_tx(&agent_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.authorize_feedback(ManagedBuffer::from("job_1"), user_addr.clone().into());
        })
        .assert_ok();

    // FAIL 1: Submit Feedback by NON-Employer (other_user)
    b_mock
        .execute_tx(&other_user_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.submit_feedback(ManagedBuffer::from("job_1"), 1u64, BigUint::from(5u64));
        })
        .assert_user_error("Only the employer can provide feedback");

    // SUCCESS: Submit Feedback by Employer
    b_mock
        .execute_tx(&user_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.submit_feedback(ManagedBuffer::from("job_1"), 1u64, BigUint::from(5u64));
        })
        .assert_ok();

    // FAIL 2: Submit Feedback Duplicate
    b_mock
        .execute_tx(&user_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.submit_feedback(ManagedBuffer::from("job_1"), 1u64, BigUint::from(1u64));
        })
        .assert_user_error("Feedback already provided for this job");
}

#[test]
fn test_append_response_failures() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let user_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));
    let other_user = b_mock.create_user_account(&rust_biguint!(0));

    // Deploy All Registries
    let val_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        validation_registry::contract_obj,
        VAL_WASM_PATH,
    );
    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        ID_WASM_PATH,
    );
    let rep_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        reputation_registry::contract_obj,
        REP_WASM_PATH,
    );

    // Configure Reputation
    let val_addr = val_wrapper.address_ref().clone();
    let id_addr = id_wrapper.address_ref().clone();
    b_mock
        .execute_tx(&owner_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.set_validation_contract_address(ManagedAddress::from(val_addr));
            sc.set_identity_contract_address(ManagedAddress::from(id_addr));
        })
        .assert_ok();

    // Setup Agent
    b_mock
        .execute_tx(&owner_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.agent_token_id()
                .set_token_id(TokenIdentifier::from("AGENT-123456"));
        })
        .assert_ok();
    b_mock.set_esdt_local_roles(
        id_wrapper.address_ref(),
        b"AGENT-123456",
        &[EsdtLocalRole::NftCreate, EsdtLocalRole::NftUpdateAttributes],
    );
    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-01"),
                ManagedBuffer::from("uri"),
                ManagedBuffer::from("pk"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    // 1. Create Job & Verify & Authorize
    b_mock
        .execute_tx(&user_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.init_job(ManagedBuffer::from("job_1"), 1u64);
        })
        .assert_ok();
    b_mock
        .execute_tx(&owner_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.verify_job(ManagedBuffer::from("job_1"));
        })
        .assert_ok();
    b_mock
        .execute_tx(&agent_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.authorize_feedback(ManagedBuffer::from("job_1"), user_addr.clone().into());
        })
        .assert_ok();

    // 2. Submit Feedback (User)
    b_mock
        .execute_tx(&user_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.submit_feedback(ManagedBuffer::from("job_1"), 1u64, BigUint::from(5u64));
        })
        .assert_ok();

    // 3. FAIL: Response by NON-Owner (other_user)
    b_mock
        .execute_tx(&other_user, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.append_response(ManagedBuffer::from("job_1"), ManagedBuffer::from("Thanks!"));
        })
        .assert_user_error("Only the agent owner can respond");
}
