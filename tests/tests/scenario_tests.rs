use common::structs::JobStatus;
use multiversx_sc::proxy_imports::OptionalValue;
use multiversx_sc::types::{BigUint, ManagedAddress, ManagedBuffer};
use multiversx_sc_scenario::api::StaticApi;
use mx_8004_tests::{constants::*, setup::AgentTestState};

// ============================================
// 1. Deploy
// ============================================

#[test]
fn test_deploy_all_contracts() {
    let state = AgentTestState::new();
    // All 3 contracts deployed in new() — addresses are non-zero
    assert_ne!(
        state.identity_sc,
        multiversx_sc::types::ManagedAddress::<StaticApi>::zero()
    );
    assert_ne!(
        state.validation_sc,
        multiversx_sc::types::ManagedAddress::<StaticApi>::zero()
    );
    assert_ne!(
        state.reputation_sc,
        multiversx_sc::types::ManagedAddress::<StaticApi>::zero()
    );
}

// ============================================
// 2. Register Agent
// ============================================

#[test]
fn test_register_agent() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![(b"key1", b"val1")],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    // Verify agent details stored
    let details = state.query_agent_details(1);
    assert_eq!(details.name, ManagedBuffer::<StaticApi>::from(b"TestAgent"));
    assert_eq!(
        details.public_key,
        ManagedBuffer::<StaticApi>::from(b"pubkey123")
    );

    // Verify owner
    let owner = state.query_agent_owner(1);
    assert_eq!(owner, AGENT_OWNER.to_managed_address());

    // Verify metadata
    let meta = state.query_metadata(1, b"key1");
    assert!(meta.is_some());

    // Verify service config
    let svc = state.query_service_config(1, 1);
    assert!(svc.is_some());
}

// ============================================
// 3. Register Agent Duplicate
// ============================================

#[test]
fn test_register_agent_duplicate() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.register_agent_expect_err(
        &AGENT_OWNER,
        b"TestAgent2",
        b"https://agent2.example.com",
        b"pubkey456",
        "Agent already registered for this address",
    );
}

// ============================================
// 4. Update Agent (requires NFT transfer + Ed25519 sig)
// ============================================

// updateAgent requires Ed25519 signature verification at VM level.
// We test the error paths (wrong NFT owner) here. Full Ed25519 flow needs chain-simulator.

#[test]
fn test_update_agent_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    // CLIENT tries to update agent 1 (owned by AGENT_OWNER) -> error
    // CLIENT doesn't have the NFT, so we need to give them one to even try
    // Instead, test with wrong NFT token by trying from a non-owner
    state.update_agent_expect_err(
        &CLIENT,
        1,
        b"NewName",
        b"https://new.uri",
        b"newpubkey",
        "insufficient funds",
    );
}

// ============================================
// 5. Set Metadata
// ============================================

#[test]
fn test_set_metadata() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.set_metadata(&AGENT_OWNER, 1, vec![(b"desc", b"A cool agent")]);

    let meta = state.query_metadata(1, b"desc");
    assert!(meta.is_some());
}

// ============================================
// 6. Remove Metadata
// ============================================

#[test]
fn test_remove_metadata() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![(b"key1", b"val1")],
        vec![],
    );

    // Confirm exists
    let meta = state.query_metadata(1, b"key1");
    assert!(meta.is_some());

    // Remove
    state.remove_metadata(&AGENT_OWNER, 1, vec![b"key1"]);

    // Confirm gone
    let meta = state.query_metadata(1, b"key1");
    assert!(meta.is_none());
}

// ============================================
// 7. Set Service Configs
// ============================================

#[test]
fn test_set_service_configs() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.set_service_configs(&AGENT_OWNER, 1, vec![(42u32, 500u64, b"USDC-abcdef", 0u64)]);

    let svc = state.query_service_config(1, 42);
    assert!(svc.is_some());
}

// ============================================
// 8. Remove Service Configs
// ============================================

#[test]
fn test_remove_service_configs() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    // Confirm exists
    let svc = state.query_service_config(1, 1);
    assert!(svc.is_some());

    // Remove
    state.remove_service_configs(&AGENT_OWNER, 1, vec![1]);

    // Confirm gone
    let svc = state.query_service_config(1, 1);
    assert!(svc.is_none());
}

// ============================================
// 9. Init Job with Payment
// ============================================

#[test]
fn test_init_job_with_payment() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    state.init_job_with_payment(
        &CLIENT,
        b"job1",
        1, // agent_nonce
        1, // service_id
        "USDC-abcdef",
        0,
        100, // amount matching service price
    );

    let job = state.query_job_data(b"job1");
    assert!(job.is_some());
    if let OptionalValue::Some(data) = job {
        assert_eq!(data.status, JobStatus::New);
        assert_eq!(data.agent_nonce, 1);
    }
}

// ============================================
// 10. Init Job (no service, free)
// ============================================

#[test]
fn test_init_job_no_service() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_free", 1, None);

    let job = state.query_job_data(b"job_free");
    assert!(job.is_some());
}

// ============================================
// 11. Init Job Invalid Payment
// ============================================

#[test]
fn test_init_job_invalid_payment() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    // Wrong amount (too low)
    state.init_job_with_payment_expect_err(
        &CLIENT,
        b"job_bad",
        1,
        1,
        "USDC-abcdef",
        0,
        50, // insufficient
        "Insufficient payment",
    );
}

// ============================================
// 12. Submit Proof
// ============================================

#[test]
fn test_submit_proof() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_proof", 1, None);
    state.submit_proof(&WORKER, b"job_proof", b"proof_data_here");

    let job = state.query_job_data(b"job_proof");
    if let OptionalValue::Some(data) = job {
        assert_eq!(data.status, JobStatus::Pending);
        assert_eq!(
            data.proof,
            ManagedBuffer::<StaticApi>::from(b"proof_data_here")
        );
    }
}

// ============================================
// 13. Verify Job
// ============================================

#[test]
fn test_verify_job() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_verify", 1, None);
    state.submit_proof(&WORKER, b"job_verify", b"proof123");
    state.verify_job(b"job_verify");

    assert!(state.query_is_job_verified(b"job_verify"));
    let job = state.query_job_data(b"job_verify");
    if let OptionalValue::Some(data) = job {
        assert_eq!(data.status, JobStatus::Verified);
    }
}

// ============================================
// 14. Verify Job Not Owner
// ============================================

#[test]
fn test_verify_job_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_notowner", 1, None);
    state.submit_proof(&WORKER, b"job_notowner", b"proof");

    // Non-owner tries to verify
    state.verify_job_expect_err(
        &CLIENT,
        b"job_notowner",
        "Endpoint can only be called by owner",
    );
}

// ============================================
// 15. Clean Old Jobs
// ============================================

#[test]
fn test_clean_old_jobs() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    // Set block timestamp to 0
    state.world.current_block().block_timestamp_millis(0);
    state.init_job(&CLIENT, b"job_old", 1, None);

    // Advance time by 4 days (> 3 days threshold)
    let four_days_ms: u64 = 4 * 24 * 60 * 60 * 1000;
    state
        .world
        .current_block()
        .block_timestamp_millis(four_days_ms);

    state.clean_old_jobs(vec![b"job_old"]);

    // Job should be cleaned
    let job = state.query_job_data(b"job_old");
    assert!(job.is_none());
}

// ============================================
// 16. Full Feedback Flow
// ============================================

#[test]
fn test_full_feedback_flow() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_fb", 1, None);
    state.submit_proof(&WORKER, b"job_fb", b"proof");
    state.verify_job(b"job_fb");

    // Agent owner authorizes CLIENT to give feedback
    state.authorize_feedback(&AGENT_OWNER, b"job_fb", &CLIENT);
    assert!(state.query_is_feedback_authorized(b"job_fb", &CLIENT));

    // CLIENT submits feedback (rating: 80)
    state.submit_feedback(&CLIENT, b"job_fb", 1, 80);

    // Verify reputation updated
    let score = state.query_reputation_score(1);
    assert_eq!(score, BigUint::<StaticApi>::from(80u64));

    let total = state.query_total_jobs(1);
    assert_eq!(total, 1u64);

    assert!(state.query_has_given_feedback(b"job_fb"));
}

// ============================================
// 17. Feedback Guards
// ============================================

#[test]
fn test_feedback_guards() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_guard", 1, None);
    state.submit_proof(&WORKER, b"job_guard", b"proof");
    state.verify_job(b"job_guard");

    // Feedback without authorization -> error
    state.submit_feedback_expect_err(
        &CLIENT,
        b"job_guard",
        1,
        80,
        "Feedback not authorized by agent",
    );

    // Authorize and submit
    state.authorize_feedback(&AGENT_OWNER, b"job_guard", &CLIENT);
    state.submit_feedback(&CLIENT, b"job_guard", 1, 90);

    // Duplicate feedback -> error
    state.submit_feedback_expect_err(
        &CLIENT,
        b"job_guard",
        1,
        90,
        "Feedback already provided for this job",
    );
}

// ============================================
// 18. Full Lifecycle E2E
// ============================================

#[test]
fn test_full_lifecycle() {
    let mut state = AgentTestState::new();

    // 1. Register agent with metadata and service config
    state.register_agent(
        &AGENT_OWNER,
        b"FullAgent",
        b"https://full.agent.com",
        b"pubkey_full",
        vec![(b"category", b"AI"), (b"version", b"1.0")],
        vec![(1u32, 200u64, b"USDC-abcdef", 0u64)],
    );

    // 2. Init job with payment
    state.init_job_with_payment(&CLIENT, b"lifecycle_job", 1, 1, "USDC-abcdef", 0, 200);

    // 3. Submit proof
    state.submit_proof(&WORKER, b"lifecycle_job", b"proof_lifecycle");

    // 4. Verify job (owner)
    state.verify_job(b"lifecycle_job");
    assert!(state.query_is_job_verified(b"lifecycle_job"));

    // 5. Authorize feedback
    state.authorize_feedback(&AGENT_OWNER, b"lifecycle_job", &CLIENT);

    // 6. Submit feedback
    state.submit_feedback(&CLIENT, b"lifecycle_job", 1, 95);
    assert_eq!(
        state.query_reputation_score(1),
        BigUint::<StaticApi>::from(95u64)
    );

    // 7. Append response
    state.append_response(&AGENT_OWNER, b"lifecycle_job", b"https://response.uri");
    let response = state.query_agent_response(b"lifecycle_job");
    assert_eq!(
        response,
        ManagedBuffer::<StaticApi>::from(b"https://response.uri")
    );
}

// ============================================
// 19. Upgrade All Contracts
// ============================================

#[test]
fn test_upgrade_all_contracts() {
    let mut state = AgentTestState::new();

    // Register agent before upgrade to verify state persists
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![(b"key1", b"val1")],
        vec![],
    );

    // Upgrade all 3
    state.upgrade_identity();
    state.upgrade_validation();
    state.upgrade_reputation();

    // Verify state persists after upgrade
    let details = state.query_agent_details(1);
    assert_eq!(details.name, ManagedBuffer::<StaticApi>::from(b"TestAgent"));
    let owner = state.query_agent_owner(1);
    assert_eq!(owner, AGENT_OWNER.to_managed_address());
}

// ============================================
// 20. Issue Token (already issued error)
// ============================================

#[test]
fn test_issue_token_already_issued() {
    let mut state = AgentTestState::new();
    // Token is already set via whitebox in new(), so issuing again should fail
    state.issue_token_expect_err("Token already issued");
}

// ============================================
// 21. Query Agent Token ID
// ============================================

#[test]
fn test_query_agent_token_id() {
    let mut state = AgentTestState::new();
    let token_id = state.query_agent_token_id();
    assert_eq!(token_id, AGENT_TOKEN.to_token_identifier());
}

// ============================================
// 22. Query Agents (BiDi mapper)
// ============================================

#[test]
fn test_query_agents_bidi() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    let agents = state.query_agents();
    let entries: Vec<_> = agents.into_iter().collect();
    assert_eq!(entries.len(), 1);
    let (nonce, addr) = entries[0].clone().into_tuple();
    assert_eq!(nonce, 1u64);
    assert_eq!(addr, AGENT_OWNER.to_managed_address());
}

// ============================================
// 23. Query Agent (getAgent view)
// ============================================

#[test]
fn test_query_get_agent_view() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    let agent = state.query_agent(1);
    assert_eq!(agent.name, ManagedBuffer::<StaticApi>::from(b"TestAgent"));
    assert_eq!(
        agent.public_key,
        ManagedBuffer::<StaticApi>::from(b"pubkey123")
    );
}

// ============================================
// 24. Query Agent Metadata Bulk
// ============================================

#[test]
fn test_query_agent_metadata_bulk() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![(b"key1", b"val1"), (b"key2", b"val2")],
        vec![],
    );

    let bulk = state.query_agent_metadata_bulk(1);
    let entries: Vec<_> = bulk.into_iter().collect();
    assert_eq!(entries.len(), 2);
}

// ============================================
// 25. Query Agent Service Config Bulk
// ============================================

#[test]
fn test_query_agent_service_bulk() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![
            (1u32, 100u64, b"USDC-abcdef", 0u64),
            (2u32, 200u64, b"USDC-abcdef", 0u64),
        ],
    );

    let bulk = state.query_agent_service_bulk(1);
    let entries: Vec<_> = bulk.into_iter().collect();
    assert_eq!(entries.len(), 2);
}

// ============================================
// 26. Admin Config: setIdentityRegistryAddress (validation)
// ============================================

#[test]
fn test_set_identity_registry_address() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(IDENTITY_SC_ADDRESS.eval_to_array());
    state.set_identity_registry_address(&OWNER_ADDRESS, new_addr);
}

#[test]
fn test_set_identity_registry_address_not_owner() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(IDENTITY_SC_ADDRESS.eval_to_array());
    state.set_identity_registry_address_expect_err(
        &CLIENT,
        new_addr,
        "Endpoint can only be called by owner",
    );
}

// ============================================
// 27. Admin Config: setIdentityContractAddress (reputation)
// ============================================

#[test]
fn test_set_reputation_identity_address() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(IDENTITY_SC_ADDRESS.eval_to_array());
    state.set_reputation_identity_address(&OWNER_ADDRESS, new_addr);
}

#[test]
fn test_set_reputation_identity_address_not_owner() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(IDENTITY_SC_ADDRESS.eval_to_array());
    state.set_reputation_identity_address_expect_err(
        &CLIENT,
        new_addr,
        "Endpoint can only be called by owner",
    );
}

// ============================================
// 28. Admin Config: setValidationContractAddress (reputation)
// ============================================

#[test]
fn test_set_reputation_validation_address() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(VALIDATION_SC_ADDRESS.eval_to_array());
    state.set_reputation_validation_address(&OWNER_ADDRESS, new_addr);
}

#[test]
fn test_set_reputation_validation_address_not_owner() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(VALIDATION_SC_ADDRESS.eval_to_array());
    state.set_reputation_validation_address_expect_err(
        &CLIENT,
        new_addr,
        "Endpoint can only be called by owner",
    );
}

// ============================================
// 29. Submit Proof — Nonexistent Job
// ============================================

#[test]
fn test_submit_proof_nonexistent_job() {
    let mut state = AgentTestState::new();
    state.submit_proof_expect_err(&WORKER, b"nonexistent-job", b"proof-data", "Job not found");
}

// ============================================
// 30. Init Job — Duplicate
// ============================================

#[test]
fn test_init_job_duplicate() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job-dup", 1, None);
    state.init_job_expect_err(&CLIENT, b"job-dup", 1, None, "Job already initialized");
}

// ============================================
// 31. Init Job — Wrong Payment Token
// ============================================

#[test]
fn test_init_job_wrong_token() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    state.init_job_with_wrong_token_expect_err(
        &CLIENT,
        b"job-wrong-tok",
        1,
        1,
        "WRONG-abcdef",
        0,
        100,
        "Invalid payment token",
    );
}

// ============================================
// 32. Set Metadata — Not Agent Owner
// ============================================

#[test]
fn test_set_metadata_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.set_metadata_expect_err(
        &CLIENT,
        1,
        vec![(b"key1", b"val1")],
        "Only the agent owner can perform this action",
    );
}

// ============================================
// 33. Set Service Configs — Not Agent Owner
// ============================================

#[test]
fn test_set_service_configs_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.set_service_configs_expect_err(
        &CLIENT,
        1,
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
        "Only the agent owner can perform this action",
    );
}

// ============================================
// 34. Remove Metadata — Not Agent Owner
// ============================================

#[test]
fn test_remove_metadata_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![(b"key1", b"val1")],
        vec![],
    );

    state.remove_metadata_expect_err(
        &CLIENT,
        1,
        vec![b"key1"],
        "Only the agent owner can perform this action",
    );
}

// ============================================
// 35. Remove Service Configs — Not Agent Owner
// ============================================

#[test]
fn test_remove_service_configs_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    state.remove_service_configs_expect_err(
        &CLIENT,
        1,
        vec![1],
        "Only the agent owner can perform this action",
    );
}

// ============================================
// 36. Authorize Feedback — Not Agent Owner
// ============================================

#[test]
fn test_authorize_feedback_not_agent_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job-auth", 1, None);
    state.submit_proof(&WORKER, b"job-auth", b"proof");
    state.verify_job(b"job-auth");

    // CLIENT (not agent owner) tries to authorize feedback
    state.authorize_feedback_expect_err(
        &CLIENT,
        b"job-auth",
        &CLIENT,
        "Only the agent owner can perform this action",
    );
}

// ============================================
// 37. Submit Feedback — Job Not Verified
// ============================================

#[test]
fn test_submit_feedback_job_not_verified() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job-unverified", 1, None);
    // Don't verify — just try feedback
    state.submit_feedback_expect_err(&CLIENT, b"job-unverified", 1, 80, "Job not verified");
}

// ============================================
// 38. Submit Feedback — Wrong Caller (not employer)
// ============================================

#[test]
fn test_submit_feedback_not_employer() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job-wrong-caller", 1, None);
    state.submit_proof(&WORKER, b"job-wrong-caller", b"proof");
    state.verify_job(b"job-wrong-caller");

    // Authorize CLIENT but WORKER tries to submit
    state.authorize_feedback(&AGENT_OWNER, b"job-wrong-caller", &CLIENT);
    state.submit_feedback_expect_err(
        &WORKER,
        b"job-wrong-caller",
        1,
        80,
        "Only the employer can provide feedback",
    );
}

// ============================================
// 39. Append Response — Not Agent Owner
// ============================================

#[test]
fn test_append_response_not_agent_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job-resp", 1, None);
    state.submit_proof(&WORKER, b"job-resp", b"proof");
    state.verify_job(b"job-resp");

    // CLIENT (not agent owner) tries to append response
    state.append_response_expect_err(
        &CLIENT,
        b"job-resp",
        b"https://response.uri",
        "Only the agent owner can perform this action",
    );
}

// ============================================
// 40. Clean Old Jobs — Job Not Old Enough
// ============================================

#[test]
fn test_clean_old_jobs_not_old_enough() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.world.current_block().block_timestamp_millis(0);
    state.init_job(&CLIENT, b"job_recent", 1, None);

    // Advance only 1 day (< 3 days threshold)
    let one_day_ms: u64 = 1 * 24 * 60 * 60 * 1000;
    state
        .world
        .current_block()
        .block_timestamp_millis(one_day_ms);

    state.clean_old_jobs(vec![b"job_recent"]);

    // Job should still exist
    let job = state.query_job_data(b"job_recent");
    assert!(job.is_some(), "Job should not be cleaned — not old enough");
}

// ============================================
// 41. Update Agent — Invalid NFT (wrong nonce)
// ============================================

#[test]
fn test_update_agent_invalid_nft() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    // Try updating with wrong NFT nonce (2 doesn't exist)
    state.update_agent_expect_err(
        &AGENT_OWNER,
        2,
        b"NewName",
        b"https://new.uri",
        b"newpubkey",
        "insufficient funds",
    );
}

// ============================================
// 43a. Update Agent — happy path (basic)
// ============================================

#[test]
#[ignore] // Requires ESDTMetaDataRecreate VM mock (not in official SDK yet)
fn test_update_agent() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    // Update name, uri, and public key
    state.update_agent_raw(
        &AGENT_OWNER,
        1,
        b"UpdatedAgent",
        b"https://updated.example.com",
        b"newpubkey",
        None,
        None,
    );

    // Agent owner preserved after update
    let owner = state.query_agent_owner(1);
    assert_eq!(owner, ManagedAddress::from(AGENT_OWNER.to_address()),);
}

// ============================================
// 43b. Update Agent — with metadata and services
// ============================================

#[test]
#[ignore] // Requires ESDTMetaDataRecreate VM mock (not in official SDK yet)
fn test_update_agent_with_meta_and_services() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.update_agent_raw(
        &AGENT_OWNER,
        1,
        b"UpdatedAgent",
        b"https://updated.example.com",
        b"newpubkey",
        Some(vec![(b"bio", b"Updated bio")]),
        Some(vec![(1, 100, b"EGLD-000000", 0)]),
    );

    // Verify metadata was updated
    let meta = state.query_metadata(1, b"bio");
    assert!(meta.is_some());

    // Verify service config was set
    let svc = state.query_service_config(1, 1);
    assert!(svc.is_some());
}

// ============================================
// 44. Upgrade Identity Registry
// ============================================

#[test]
fn test_upgrade_identity() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.upgrade_identity();

    let details = state.query_agent_details(1);
    assert_eq!(details.name, ManagedBuffer::<StaticApi>::from(b"TestAgent"));
}

// ============================================
// 45. Upgrade Validation Registry
// ============================================

#[test]
fn test_upgrade_validation() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_upgrade", 1, None);
    state.upgrade_validation();

    let job = state.query_job_data(b"job_upgrade");
    assert!(job.is_some(), "Job should persist after upgrade");
}

// ============================================
// 46. Upgrade Reputation Registry
// ============================================

#[test]
fn test_upgrade_reputation() {
    let mut state = AgentTestState::new();
    state.upgrade_reputation();

    // After upgrade, config should still be intact
    let validation_addr = state.query_validation_contract_address();
    assert_eq!(
        validation_addr,
        ManagedAddress::<StaticApi>::from(VALIDATION_SC_ADDRESS.eval_to_array())
    );
}

// ============================================
// 47. Query Reputation Contract Addresses
// ============================================

#[test]
fn test_query_reputation_contract_addresses() {
    let mut state = AgentTestState::new();

    let validation_addr = state.query_validation_contract_address();
    assert_eq!(
        validation_addr,
        ManagedAddress::<StaticApi>::from(VALIDATION_SC_ADDRESS.eval_to_array())
    );

    let identity_addr = state.query_identity_contract_address();
    assert_eq!(
        identity_addr,
        ManagedAddress::<StaticApi>::from(IDENTITY_SC_ADDRESS.eval_to_array())
    );
}
