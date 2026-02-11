# MX-8004: Trustless Agents Standard Specification

## Overview

Three smart contracts forming a decentralized agent identity, job validation, and reputation system on MultiversX. Contracts communicate via **cross-contract storage reads** (`storage_mapper_from_address`) — no async calls.

---

## 1. Identity Registry

Manages agent identities as soulbound (non-transferable) NFTs.

### 1.1 Endpoints

| Endpoint | Access | Description |
|---|---|---|
| `init()` | deploy | No-op constructor |
| `upgrade()` | upgrade | No-op |
| `issue_token(name, ticker)` | owner, payable EGLD | Issues the NFT collection; can only be called once |
| `register_agent(name, uri, public_key, metadata?, services?)` | anyone | Mints soulbound NFT, stores agent data, sends NFT to caller |
| `update_agent(new_name, new_uri, new_public_key, signature, metadata?, services?)` | agent owner, payable NFT | Transfer-execute: send NFT in, verify Ed25519 signature over `sha256(new_public_key)`, update on-chain data via `esdt_metadata_recreate`, return NFT |
| `set_metadata(nonce, entries)` | agent owner | Upsert key-value metadata in `MapMapper` |
| `set_service_configs(nonce, configs)` | agent owner | Upsert service pricing in `MapMapper<u32, Payment>`. `price = 0` removes the service |
| `remove_metadata(nonce, keys)` | agent owner | Remove metadata entries by key (`MultiValueEncoded<ManagedBuffer>`) |
| `remove_service_configs(nonce, service_ids)` | agent owner | Remove service configs by ID (`MultiValueEncoded<u32>`) |

### 1.2 Views

| View | Returns |
|---|---|
| `get_agent(nonce)` | `AgentDetails { name, public_key }` |
| `get_agent_owner(nonce)` | `ManagedAddress` |
| `get_metadata(nonce, key)` | `OptionalValue<ManagedBuffer>` |
| `get_agent_service_config(nonce, service_id)` | `OptionalValue<EgldOrEsdtTokenPayment>` |
| `get_agent_token_id()` | `NonFungibleTokenMapper` (raw) |
| `get_agent_id()` | `BiDiMapper<u64, ManagedAddress>` (raw) |
| `get_agent_details(nonce)` | `SingleValueMapper<AgentDetails>` (raw) |
| `get_agent_metadata(nonce)` | `MapMapper<ManagedBuffer, ManagedBuffer>` (raw) |
| `get_agent_service(nonce)` | `MapMapper<u32, Payment>` (raw) |

### 1.3 Storage

| Key | Type | Description |
|---|---|---|
| `agentTokenId` | `NonFungibleTokenMapper` | NFT collection token ID |
| `agents` | `BiDiMapper<u64, ManagedAddress>` | Nonce <-> owner bidirectional map |
| `agentDetails(nonce)` | `SingleValueMapper<AgentDetails>` | Name + public key |
| `agentMetadatas(nonce)` | `MapMapper<ManagedBuffer, ManagedBuffer>` | Generic key-value metadata |
| `agentServiceConfigs(nonce)` | `MapMapper<u32, Payment>` | Service ID -> payment config |

### 1.4 Events

- `agentRegistered(owner, nonce, AgentRegisteredEventData { name, uri })`
- `agentUpdated(nonce)`
- `metadataUpdated(nonce)`
- `serviceConfigsUpdated(nonce)`

---

## 2. Validation Registry

Handles job lifecycle: initialization, proof submission, ERC-8004 validation (request/response), and cleanup.

### 2.1 Endpoints

| Endpoint | Access | Description |
|---|---|---|
| `init(identity_registry_address)` | deploy | Stores identity registry address |
| `upgrade()` | upgrade | No-op |
| `init_job(job_id, agent_nonce, service_id?)` | anyone, payable | Creates job with `New` status. If `service_id` provided, reads agent's service config from identity registry via cross-contract storage, validates payment token/nonce, requires `amount >= price`, and forwards payment to agent owner |
| `submit_proof(job_id, proof)` | anyone | Sets proof data and transitions status `New -> Pending` |
| `submit_proof_with_nft(job_id, proof)` | anyone, payable NFT | Like `submit_proof` but accepts an NFT as proof attachment |
| `validation_request(job_id, validator_address, request_uri, request_hash)` | agent owner | ERC-8004: Nominate a validator for the job. Sets status to `ValidationRequested`. Emits `validationRequestEvent` |
| `validation_response(request_hash, response, response_uri, response_hash, tag)` | nominated validator | ERC-8004: Validator submits a response (score 0-100). Sets status to `Verified`. Emits `validationResponseEvent` |
| `clean_old_jobs(job_ids)` | anyone | Removes jobs older than 3 days (259,200,000 ms) |
| `set_identity_registry_address(address)` | owner only | Update identity registry address |

### 2.2 Views

| View | Returns |
|---|---|
| `is_job_verified(job_id)` | `bool` |
| `get_job_data(job_id)` | `OptionalValue<JobData>` |
| `get_validation_status(request_hash)` | `OptionalValue<ValidationRequestData>` |
| `get_agent_validations(agent_nonce)` | `UnorderedSetMapper<ManagedBuffer>` |

### 2.3 Storage

| Key | Type |
|---|---|
| `jobData(job_id)` | `SingleValueMapper<JobData>` |
| `identityRegistryAddress` | `SingleValueMapper<ManagedAddress>` |
| `validationRequestData(request_hash)` | `SingleValueMapper<ValidationRequestData>` |
| `agentValidations(agent_nonce)` | `UnorderedSetMapper<ManagedBuffer>` |

### 2.4 Events

- `validationRequestEvent(job_id, agent_nonce, validator_address, request_uri, request_hash)`
- `validationResponseEvent(request_hash, response, response_hash, tag)`

---

## 3. Reputation Registry

Collects feedback on jobs and computes on-chain reputation scores. No pre-authorization needed — the employer who created the job can submit feedback directly.

### 3.1 Endpoints

| Endpoint | Access | Description |
|---|---|---|
| `init(validation_addr, identity_addr)` | deploy | Stores both contract addresses |
| `upgrade()` | upgrade | No-op |
| `submit_feedback(job_id, agent_nonce, rating)` | employer only | Validates: (1) job exists via cross-contract read from validation registry, (2) caller is the employer who created the job, (3) no duplicate feedback for this job. Updates cumulative moving average score |
| `append_response(job_id, response_uri)` | anyone | ERC-8004: Anyone can append a response URI to a job (e.g., agent showing refund, data aggregator tagging feedback as spam) |
| `set_identity_contract_address(address)` | owner only | Update identity registry address |
| `set_validation_contract_address(address)` | owner only | Update validation registry address |

### 3.2 Views

| View | Returns |
|---|---|
| `get_reputation_score(agent_nonce)` | `BigUint` |
| `get_total_jobs(agent_nonce)` | `u64` |
| `has_given_feedback(job_id)` | `bool` |
| `get_agent_response(job_id)` | `ManagedBuffer` |
| `get_validation_contract_address()` | `ManagedAddress` |
| `get_identity_contract_address()` | `ManagedAddress` |

### 3.3 Storage

| Key | Type |
|---|---|
| `reputationScore(agent_nonce)` | `SingleValueMapper<BigUint>` |
| `totalJobs(agent_nonce)` | `SingleValueMapper<u64>` |
| `hasGivenFeedback(job_id)` | `SingleValueMapper<bool>` |
| `agentResponse(job_id)` | `SingleValueMapper<ManagedBuffer>` |
| `validationContractAddress` | `SingleValueMapper<ManagedAddress>` |
| `identityContractAddress` | `SingleValueMapper<ManagedAddress>` |

### 3.4 Scoring Algorithm

Cumulative moving average:

```
new_score = (current_score * (total_jobs - 1) + rating) / total_jobs
```

`total_jobs` is incremented atomically before the calculation.

### 3.5 Events

- `reputationUpdated(agent_nonce, new_score)`

---

## 4. Shared Types (`common` crate)

```rust
pub struct AgentDetails<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub public_key: ManagedBuffer<M>,
}

pub struct MetadataEntry<M: ManagedTypeApi> {
    pub key: ManagedBuffer<M>,
    pub value: ManagedBuffer<M>,
}

pub struct ServiceConfigInput<M: ManagedTypeApi> {
    pub service_id: u32,
    pub price: BigUint<M>,
    pub token: TokenId<M>,
    pub nonce: u64,
}

pub struct AgentRegisteredEventData<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub uri: ManagedBuffer<M>,
}

pub enum JobStatus { New, Pending, Verified, ValidationRequested }

pub struct JobData<M: ManagedTypeApi> {
    pub status: JobStatus,
    pub proof: ManagedBuffer<M>,
    pub employer: ManagedAddress<M>,
    pub creation_timestamp: TimestampMillis,
    pub agent_nonce: u64,
}
```

---

## 5. Cross-Contract Storage Reads

All inter-contract communication uses `#[storage_mapper_from_address]` — synchronous reads from another contract's storage on the same shard. No async calls, no callbacks.

| Consumer | Source Contract | Storage Key | Mapper Type |
|---|---|---|---|
| Validation Registry | Identity Registry | `agents` | `BiDiMapper<u64, ManagedAddress>` |
| Validation Registry | Identity Registry | `agentServiceConfigs` | `MapMapper<u32, Payment>` |
| Reputation Registry | Validation Registry | `jobData` | `SingleValueMapper<JobData>` |
| Reputation Registry | Identity Registry | `agents` | `BiDiMapper<u64, ManagedAddress>` |

Defined in `common::cross_contract::CrossContractModule`.

---

## 6. Contract Interaction Flow

```
1. Owner deploys Identity Registry, calls issue_token()
2. Owner deploys Validation Registry with identity registry address
3. Owner deploys Reputation Registry with both addresses

Agent Lifecycle:
4. Agent calls register_agent() -> receives soulbound NFT
5. Client calls init_job(job_id, agent_nonce, service_id) with payment -> payment forwarded to agent owner
6. Worker calls submit_proof(job_id, proof) -> job status: Pending
7. (Optional) Agent owner calls validation_request(job_id, validator, uri, hash) -> status: ValidationRequested
8. (Optional) Validator calls validation_response(request_hash, response, uri, hash, tag) -> status: Verified
9. Client calls submit_feedback(job_id, agent_nonce, rating) -> reputation score updated
10. Anyone optionally calls append_response(job_id, uri)
```
