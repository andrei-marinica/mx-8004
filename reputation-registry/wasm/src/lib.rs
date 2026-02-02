#![no_std]
#![cfg(not(test))]

#[cfg(not(test))]
multiversx_sc_wasm_adapter::allocator!();
#[cfg(not(test))]
multiversx_sc_wasm_adapter::panic_handler!();

#[cfg(not(test))]
multiversx_sc_wasm_adapter::endpoints! {
    reputation_registry
    (
        init => init
        submit_feedback => submit_feedback
        authorize_feedback => authorize_feedback
        append_response => append_response
        reputationScore => reputation_score
        totalJobs => total_jobs
        validationContractAddress => validation_contract_address
        isFeedbackAuthorized => is_feedback_authorized
        hasGivenFeedback => has_given_feedback
        agentResponse => agent_response
    )
}

#[cfg(not(test))]
multiversx_sc_wasm_adapter::async_callback_empty! {}
