// Chain-simulator E2E tests — feature-gated behind `chain-simulator-tests`.
// Run with: cargo test -p mx-8004-tests --features chain-simulator-tests -- --test-threads=1
//
// These tests require a running MultiversX chain simulator on http://localhost:8085

#[cfg(feature = "chain-simulator-tests")]
mod cs {
    use mx_8004_tests::interact::CsInteract;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_deploy_all_cs() {
        let _ = env_logger::try_init();
        let interact = CsInteract::new().await;

        assert!(
            !interact.identity_addr.to_bech32_string().is_empty(),
            "Identity address should be non-empty"
        );
        assert!(
            !interact.validation_addr.to_bech32_string().is_empty(),
            "Validation address should be non-empty"
        );
        assert!(
            !interact.reputation_addr.to_bech32_string().is_empty(),
            "Reputation address should be non-empty"
        );
        assert!(
            !interact.agent_token_id.is_empty(),
            "Agent token ID should be set"
        );
        println!(
            "All contracts deployed successfully. Token: {}",
            interact.agent_token_id
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_register_agent_cs() {
        let _ = env_logger::try_init();
        let mut interact = CsInteract::new().await;

        let bob = interact.agent_owner.clone();
        interact
            .register_agent(&bob, b"TestAgent", b"https://agent.example.com", b"pubkey123")
            .await;

        let token_id = interact.query_agent_token_id().await;
        assert!(!token_id.is_empty(), "Token ID should exist after registration");
        println!("Agent registered and confirmed with token: {token_id}");
    }

    #[tokio::test]
    #[serial]
    async fn test_job_lifecycle_cs() {
        let _ = env_logger::try_init();
        let mut interact = CsInteract::new().await;

        let bob = interact.agent_owner.clone();
        interact
            .register_agent(&bob, b"TestAgent", b"https://agent.example.com", b"pubkey123")
            .await;

        let agent_nonce = 1u64;
        let carol = interact.client.clone();
        let worker = interact.worker.clone();

        interact.init_job(&carol, b"job-001", agent_nonce).await;
        interact
            .submit_proof(&worker, b"job-001", b"proof-data-hash")
            .await;
        interact.verify_job(b"job-001").await;

        let verified = interact.query_is_job_verified(b"job-001").await;
        assert!(verified, "Job should be verified");
        println!("Full job lifecycle (init -> proof -> verify) passed");
    }

    /// Full lifecycle including reputation feedback.
    ///
    /// This test is ignored by default because the reputation-registry uses
    /// `storage_mapper_from_address` (ManagedStorageReadFromAddress VM hook)
    /// to read job data from the validation-registry. This VM hook only works
    /// when both contracts are deployed in the same shard. On the chain simulator
    /// with 3 shards, contracts deployed at different nonces land in different
    /// shards non-deterministically, causing "Job not found" errors.
    ///
    /// The full lifecycle (including feedback) is covered by the 32 scenario tests.
    ///
    /// To run: cargo test -p mx-8004-tests --features chain-simulator-tests -- --ignored --test-threads=1
    #[tokio::test]
    #[serial]
    #[ignore = "Requires same-shard deployment; storage_mapper_from_address is shard-local"]
    async fn test_full_lifecycle_with_feedback_cs() {
        let _ = env_logger::try_init();
        let mut interact = CsInteract::new().await;

        let bob = interact.agent_owner.clone();
        interact
            .register_agent(&bob, b"TestAgent", b"https://agent.example.com", b"pubkey123")
            .await;

        let agent_nonce = 1u64;
        let carol = interact.client.clone();
        let worker = interact.worker.clone();

        // Job lifecycle
        interact.init_job(&carol, b"job-001", agent_nonce).await;
        interact
            .submit_proof(&worker, b"job-001", b"proof-data-hash")
            .await;
        interact.verify_job(b"job-001").await;

        // Reputation (cross-contract storage reads from validation)
        interact
            .authorize_feedback(&bob, b"job-001", &carol)
            .await;
        interact
            .submit_feedback(&carol, b"job-001", agent_nonce, 85)
            .await;
        interact
            .append_response(&bob, b"job-001", b"https://response.example.com/result")
            .await;

        let score = interact.query_reputation_score(agent_nonce).await;
        assert!(
            score > multiversx_sc_scenario::imports::RustBigUint::from(0u64),
            "Score should be > 0"
        );

        let total = interact.query_total_jobs(agent_nonce).await;
        assert_eq!(total, 1, "Should have 1 completed job");

        let has_feedback = interact.query_has_given_feedback(b"job-001").await;
        assert!(has_feedback, "Feedback should be recorded");

        println!("Full lifecycle with feedback passed!");
    }

    // ── Error-path tests ──

    #[tokio::test]
    #[serial]
    async fn test_duplicate_agent_registration_cs() {
        let _ = env_logger::try_init();
        let mut interact = CsInteract::new().await;

        let bob = interact.agent_owner.clone();
        interact
            .register_agent(&bob, b"TestAgent", b"https://agent.example.com", b"pubkey123")
            .await;

        // Second registration from same address should fail
        interact
            .register_agent_expect_err(
                &bob,
                b"TestAgent2",
                b"https://agent2.example.com",
                b"pubkey456",
                4,
                "Agent already registered for this address",
            )
            .await;

        println!("Duplicate agent registration correctly rejected");
    }

    #[tokio::test]
    #[serial]
    async fn test_issue_token_already_issued_cs() {
        let _ = env_logger::try_init();
        let mut interact = CsInteract::new().await;

        // Token is already issued during CsInteract::new(), second issuance should fail
        interact
            .issue_token_expect_err(4, "Token already issued")
            .await;

        println!("Duplicate token issuance correctly rejected");
    }

    #[tokio::test]
    #[serial]
    async fn test_verify_job_not_owner_cs() {
        let _ = env_logger::try_init();
        let mut interact = CsInteract::new().await;

        let bob = interact.agent_owner.clone();
        interact
            .register_agent(&bob, b"TestAgent", b"https://agent.example.com", b"pubkey123")
            .await;

        let carol = interact.client.clone();
        interact.init_job(&carol, b"job-001", 1u64).await;

        let worker = interact.worker.clone();
        interact
            .submit_proof(&worker, b"job-001", b"proof-data")
            .await;

        // Non-owner (bob) tries to verify — should fail with only_owner error
        interact
            .verify_job_expect_err(
                &bob,
                b"job-001",
                4,
                "Endpoint can only be called by owner",
            )
            .await;

        println!("Non-owner verify correctly rejected");
    }

    #[tokio::test]
    #[serial]
    async fn test_submit_proof_nonexistent_job_cs() {
        let _ = env_logger::try_init();
        let mut interact = CsInteract::new().await;

        let worker = interact.worker.clone();
        interact
            .submit_proof_expect_err(
                &worker,
                b"nonexistent-job",
                b"proof-data",
                4,
                "Job not found",
            )
            .await;

        println!("Proof for nonexistent job correctly rejected");
    }

    #[tokio::test]
    #[serial]
    async fn test_init_job_duplicate_cs() {
        let _ = env_logger::try_init();
        let mut interact = CsInteract::new().await;

        let bob = interact.agent_owner.clone();
        interact
            .register_agent(&bob, b"TestAgent", b"https://agent.example.com", b"pubkey123")
            .await;

        let carol = interact.client.clone();
        interact.init_job(&carol, b"job-001", 1u64).await;

        // Same job_id again should fail
        interact
            .init_job_expect_err(&carol, b"job-001", 1u64, 4, "Job already initialized")
            .await;

        println!("Duplicate job init correctly rejected");
    }
}
