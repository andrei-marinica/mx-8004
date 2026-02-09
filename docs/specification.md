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

Handles job lifecycle: initialization, proof submission, verification, and cleanup.

### 2.1 Endpoints

| Endpoint | Access | Description |
|---|---|---|
| `init(identity_registry_address)` | deploy | Stores identity registry address |
| `upgrade()` | upgrade | No-op |
| `init_job(job_id, agent_nonce, service_id?)` | anyone, payable | Creates job with `New` status. If `service_id` provided, reads agent's service config from identity registry via cross-contract storage, validates payment token/nonce, requires `amount >= price`, and forwards payment to agent owner |
| `submit_proof(job_id, proof)` | anyone | Sets proof data and transitions status `New -> Pending` |
| `verify_job(job_id)` | owner only | Transitions status `Pending -> Verified`, emits event |
| `clean_old_jobs(job_ids)` | anyone | Removes jobs older than 3 days (259,200,000 ms) |
| `set_identity_registry_address(address)` | owner only | Update identity registry address |

### 2.2 Views

| View | Returns |
|---|---|
| `is_job_verified(job_id)` | `bool` |
| `get_job_data(job_id)` | `OptionalValue<JobData>` |

### 2.3 Storage

| Key | Type |
|---|---|
| `jobData(job_id)` | `SingleValueMapper<JobData>` |
| `identityRegistryAddress` | `SingleValueMapper<ManagedAddress>` |

### 2.4 Events

- `jobVerified(job_id, agent_nonce, JobStatus::Verified)`

---

## 3. Reputation Registry

Collects feedback on verified jobs with authorization gates and cumulative scoring.

### 3.1 Endpoints

| Endpoint | Access | Description |
|---|---|---|
| `init(validation_addr, identity_addr)` | deploy | Stores both contract addresses |
| `upgrade()` | upgrade | No-op |
| `submit_feedback(job_id, agent_nonce, rating)` | employer only | Validates: (1) job exists and is `Verified` via cross-contract read, (2) caller is employer, (3) agent owner authorized this feedback, (4) no duplicate feedback. Updates cumulative moving average score |
| `authorize_feedback(job_id, client)` | agent owner | Agent owner authorizes a specific client address to submit feedback for a job |
| `append_response(job_id, response_uri)` | agent owner | Agent owner attaches a response URI to a job |
| `set_identity_contract_address(address)` | owner only | Update identity registry address |
| `set_validation_contract_address(address)` | owner only | Update validation registry address |

### 3.2 Views

| View | Returns |
|---|---|
| `get_reputation_score(agent_nonce)` | `BigUint` |
| `get_total_jobs(agent_nonce)` | `u64` |
| `has_given_feedback(job_id)` | `bool` |
| `is_feedback_authorized(job_id, client)` | `bool` |
| `get_agent_response(job_id)` | `ManagedBuffer` |
| `get_validation_contract_address()` | `ManagedAddress` |
| `get_identity_contract_address()` | `ManagedAddress` |

### 3.3 Storage

| Key | Type |
|---|---|
| `reputationScore(agent_nonce)` | `SingleValueMapper<BigUint>` |
| `totalJobs(agent_nonce)` | `SingleValueMapper<u64>` |
| `hasGivenFeedback(job_id)` | `SingleValueMapper<bool>` |
| `isFeedbackAuthorized(job_id, client)` | `SingleValueMapper<bool>` |
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

pub enum JobStatus { New, Pending, Verified }

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
7. Contract owner calls verify_job(job_id) -> job status: Verified
8. Agent owner calls authorize_feedback(job_id, client_address)
9. Client calls submit_feedback(job_id, agent_nonce, rating) -> reputation score updated
10. Agent owner optionally calls append_response(job_id, uri)
```
