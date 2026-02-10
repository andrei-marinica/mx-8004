#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod errors;
pub mod events;
pub mod storage;

use errors::*;
use storage::{EscrowData, EscrowStatus};

/// ACP Escrow Contract — locks funds for agent jobs, releases on proof verification,
/// refunds if deadline passes without verified proof.
///
/// Follows Checks-Effects-Interactions pattern throughout.
#[multiversx_sc::contract]
pub trait EscrowContract:
    common::cross_contract::CrossContractModule + storage::StorageModule + events::EventsModule
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

    /// Deposit funds into escrow for a specific job.
    /// Accepts EGLD or any ESDT token.
    /// `deadline` is a Unix timestamp (seconds) after which a refund is allowed.
    #[payable("*")]
    #[endpoint(deposit)]
    fn deposit(
        &self,
        job_id: ManagedBuffer,
        receiver: ManagedAddress,
        poa_hash: ManagedBuffer,
        deadline: TimestampSeconds,
    ) {
        let payment = self.call_value().egld_or_single_esdt();
        require!(payment.amount > 0u64, ERR_ZERO_DEPOSIT);

        let current_timestamp = self.blockchain().get_block_timestamp_seconds();
        require!(deadline > current_timestamp, ERR_DEADLINE_IN_PAST);

        let escrow_mapper = self.escrow_data(&job_id);
        require!(escrow_mapper.is_empty(), ERR_ESCROW_ALREADY_EXISTS);

        let caller = self.blockchain().get_caller();

        let escrow = EscrowData {
            employer: caller.clone(),
            receiver,
            token_id: payment.token_identifier.clone(),
            token_nonce: payment.token_nonce,
            amount: payment.amount.clone(),
            poa_hash,
            deadline,
            status: EscrowStatus::Active,
        };

        // Effects: store escrow
        escrow_mapper.set(&escrow);

        self.escrow_deposited_event(&job_id, &caller, payment.amount);
    }

    /// Release escrowed funds to the receiver.
    /// Only callable by the employer. Job must be verified in the ValidationRegistry.
    #[endpoint(release)]
    fn release(&self, job_id: ManagedBuffer) {
        let escrow_mapper = self.escrow_data(&job_id);
        require!(!escrow_mapper.is_empty(), ERR_ESCROW_NOT_FOUND);

        let mut escrow = escrow_mapper.get();
        require!(escrow.status == EscrowStatus::Active, ERR_ALREADY_SETTLED);

        let caller = self.blockchain().get_caller();
        require!(caller == escrow.employer, ERR_NOT_EMPLOYER);

        // Cross-contract check: read job status from validation-registry
        let validation_addr = self.validation_contract_address().get();
        let job_mapper = self.external_job_data(validation_addr, &job_id);
        require!(!job_mapper.is_empty(), ERR_ESCROW_NOT_FOUND);

        let job_data = job_mapper.get();
        require!(
            job_data.status == common::structs::JobStatus::Verified,
            ERR_JOB_NOT_VERIFIED
        );

        // Effects: mark as released BEFORE interactions
        escrow.status = EscrowStatus::Released;
        let receiver = escrow.receiver.clone();
        let amount = escrow.amount.clone();
        let token_id = escrow.token_id.clone();
        let token_nonce = escrow.token_nonce;
        escrow_mapper.set(&escrow);

        // Interactions: transfer funds to receiver
        self.tx()
            .to(&receiver)
            .egld_or_single_esdt(&token_id, token_nonce, &amount)
            .transfer();

        self.escrow_released_event(&job_id, &receiver, amount);
    }

    /// Refund escrowed funds to the employer if the deadline has passed.
    /// Anyone can call this (allows automated cleanup).
    #[endpoint(refund)]
    fn refund(&self, job_id: ManagedBuffer) {
        let escrow_mapper = self.escrow_data(&job_id);
        require!(!escrow_mapper.is_empty(), ERR_ESCROW_NOT_FOUND);

        let mut escrow = escrow_mapper.get();
        require!(escrow.status == EscrowStatus::Active, ERR_ALREADY_SETTLED);

        let current_timestamp = self.blockchain().get_block_timestamp_seconds();
        require!(current_timestamp > escrow.deadline, ERR_DEADLINE_NOT_PASSED);

        // Effects: mark as refunded BEFORE interactions
        escrow.status = EscrowStatus::Refunded;
        let employer = escrow.employer.clone();
        let amount = escrow.amount.clone();
        let token_id = escrow.token_id.clone();
        let token_nonce = escrow.token_nonce;
        escrow_mapper.set(&escrow);

        // Interactions: transfer funds back to employer
        self.tx()
            .to(&employer)
            .egld_or_single_esdt(&token_id, token_nonce, &amount)
            .transfer();

        self.escrow_refunded_event(&job_id, &employer, amount);
    }
}
