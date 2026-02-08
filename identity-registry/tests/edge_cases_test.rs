use identity_registry::*;
use multiversx_sc::types::{EsdtLocalRole, ManagedBuffer, TokenIdentifier};
use multiversx_sc_scenario::imports::OptionalValue;
use multiversx_sc_scenario::rust_biguint;
use multiversx_sc_scenario::testing_framework::BlockchainStateWrapper;

const ID_WASM_PATH: &str = "output/identity-registry.wasm";

#[test]
fn test_register_duplicate_agent() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));

    // 1. Deploy Identity Registry
    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        ID_WASM_PATH,
    );

    // 2. Setup Token
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

    // 3. Register Agent (First time - Success)
    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-01"),
                ManagedBuffer::from("ipfs://manifest"),
                ManagedBuffer::from("pubkey"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    // 4. Register Agent (Second time - Fail)
    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-02"), // Different name
                ManagedBuffer::from("ipfs://manifest"),
                ManagedBuffer::from("pubkey"),
                OptionalValue::None,
            );
        })
        .assert_user_error("Agent already registered for this address");
}

#[test]
fn test_update_agent_invalid_payment() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));

    // 1. Deploy Identity Registry
    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        ID_WASM_PATH,
    );

    // 2. Setup Token
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

    // 3. Register Agent
    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-01"),
                ManagedBuffer::from("ipfs://manifest"),
                ManagedBuffer::from("pubkey"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    // 4. Attempt Update without Payment (Direct Call should fail logic if payment missing, but here check wrong token)
    // Sending different token
    let fake_token_id = b"FAKE-123456";
    b_mock.set_esdt_balance(&agent_addr, fake_token_id, &rust_biguint!(1));

    b_mock
        .execute_esdt_transfer(
            &agent_addr,
            &id_wrapper,
            fake_token_id,
            0u64,
            &rust_biguint!(1),
            |sc| {
                sc.update_agent(
                    ManagedBuffer::from("new_uri"),
                    ManagedBuffer::from("new_key"),
                    OptionalValue::None,
                );
            },
        )
        .assert_user_error("Invalid NFT sent"); // Contract checks payment.token_identifier == AGENT-123456
}
