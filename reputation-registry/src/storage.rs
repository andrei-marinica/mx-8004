multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub use common::structs::{JobData, JobStatus};

#[multiversx_sc::module]
pub trait StorageModule: common::cross_contract::CrossContractModule {
    // ── Local storage ──

    #[view(get_reputation_score)]
    #[storage_mapper("reputationScore")]
    fn reputation_score(&self, agent_nonce: u64) -> SingleValueMapper<BigUint>;

    #[view(get_total_jobs)]
    #[storage_mapper("totalJobs")]
    fn total_jobs(&self, agent_nonce: u64) -> SingleValueMapper<u64>;

    #[view(get_validation_contract_address)]
    #[storage_mapper("validationContractAddress")]
    fn validation_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(get_identity_contract_address)]
    #[storage_mapper("identityContractAddress")]
    fn identity_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(has_given_feedback)]
    #[storage_mapper("hasGivenFeedback")]
    fn has_given_feedback(&self, job_id: ManagedBuffer) -> SingleValueMapper<bool>;

    #[view(get_agent_response)]
    #[storage_mapper("agentResponse")]
    fn agent_response(&self, job_id: ManagedBuffer) -> SingleValueMapper<ManagedBuffer>;
}
