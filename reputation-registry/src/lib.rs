#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod config;
mod errors;
mod events;
pub mod storage;
mod utils;

use errors::*;

#[multiversx_sc::contract]
pub trait ReputationRegistry:
    common::cross_contract::CrossContractModule
    + storage::StorageModule
    + events::EventsModule
    + config::ConfigModule
    + utils::UtilsModule
{
    #[init]
    fn init(
        &self,
        validation_contract_address: ManagedAddress,
        identity_contract_address: ManagedAddress,
    ) {
        self.validation_contract_address()
            .set(&validation_contract_address);
        self.identity_contract_address()
            .set(&identity_contract_address);
    }

    #[upgrade]
    fn upgrade(&self) {}

    /// Submit feedback for a job. Caller must be the employer who created the job.
    /// Job must have a validation response recorded (no pre-authorization needed).
    #[endpoint(submit_feedback)]
    fn submit_feedback(&self, job_id: ManagedBuffer, agent_nonce: u64, rating: BigUint) {
        let caller = self.blockchain().get_caller();
        let validation_addr = self.validation_contract_address().get();

        // 1. Authenticity: Read job data directly from validation-registry storage
        let job_mapper = self.external_job_data(validation_addr, &job_id);
        require!(!job_mapper.is_empty(), ERR_JOB_NOT_FOUND);
        let job_data = job_mapper.get();

        // 2. Frontrunning Protection: Verify caller is the employer
        require!(caller == job_data.employer, ERR_NOT_EMPLOYER);

        // 3. Duplicate Prevention
        require!(
            !self.has_given_feedback(job_id.clone()).get(),
            ERR_FEEDBACK_ALREADY_PROVIDED
        );

        let new_score = self.calculate_new_score(agent_nonce, rating);

        self.reputation_score(agent_nonce).set(&new_score);
        self.has_given_feedback(job_id).set(true);

        self.reputation_updated_event(agent_nonce, new_score);
    }

    /// ERC-8004: Anyone can append a response to feedback (e.g., agent showing refund,
    /// data aggregator tagging feedback as spam).
    #[endpoint(append_response)]
    fn append_response(&self, job_id: ManagedBuffer, response_uri: ManagedBuffer) {
        let validation_addr = self.validation_contract_address().get();
        let job_mapper = self.external_job_data(validation_addr, &job_id);
        require!(!job_mapper.is_empty(), ERR_JOB_NOT_FOUND);

        // Per ERC-8004: anyone can append responses — no caller check
        self.agent_response(job_id).set(response_uri);
    }
}
