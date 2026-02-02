# Identity NFT & Soulbound Improvements Implementation Plan

**Goal:** Transform the `IdentityRegistry` from a simple storage-based registry to a native MultiversX NFT collection where each agent is represented by a Soulbound Dynamic NFT.

**Architecture:**
- **Token Type**: Non-Fungible ESDT.
- **Issuance**: Admin endpoint `issue_token` triggers an async call to the metachain.
- **Soulbound**: The contract holds the `ESDTRoleNFTTransfer` role and does not expose a transfer endpoint. Users receive the NFT upon registration but cannot move it.
- **Attributes**: `AgentDetails` struct will be encoded as NFT attributes.
- **Dynamic Updates**: `update_agent` modifies the NFT attributes on-chain using `nft_update_attributes`.

**Tech Stack:**
- `multiversx-sc` 0.50+
- `DefaultIssueCallbacksModule`
- `NonFungibleTokenMapper`

---

### Task 1: Contract Infrastructure

**Files:**
- Modify: `identity-registry/Cargo.toml`
- Modify: `identity-registry/src/lib.rs`

**Task Description:**
1.  Add `multiversx-sc-modules = "0.54.0"` to `identity-registry/Cargo.toml`.
2.  Import `multiversx_sc_modules::default_issue_callbacks`.
3.  Add `DefaultIssueCallbacksModule` as a parent trait to the contract.
4.  Replace `agent_nft_token_id` mapper with `NonFungibleTokenMapper`.

```rust
#[multiversx_sc::contract]
pub trait IdentityRegistry: multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule {
    #[view(getAgentTokenId)]
    #[storage_mapper("agentTokenId")]
    fn agent_token_id(&self) -> NonFungibleTokenMapper;
}
```

---

### Task 2: Token Issuance Flow

**Modify:** `identity-registry/src/lib.rs`
1.  Implement `issue_token` endpoint (Payable for 0.05 EGLD).
2.  Call `self.agent_token_id().issue_and_set_all_roles(...)`.
3.  Ensure `register_agent` and `update_agent` are locked until `agent_token_id` is set.

---

### Task 3: NFT Registration (Minting)

**Modify:** `identity-registry/src/lib.rs`
1.  Update `AgentDetails` to be compatible with `NestedEncode/Decode`.
2.  In `register_agent`:
    -   Generate a new nonce from `agent_token_nonce`.
    -   Call `self.send().nft_create(...)`.
    -   Include `AgentDetails` as attributes.
    -   Send NFT to the caller.

---

### Task 4: Dynamic Attribute Updates

**Modify:** `identity-registry/src/lib.rs`
1.  In `update_agent`:
    -   Verify ownership of the NFT nonce.
    -   Encode new `AgentDetails`.
    -   Call `self.send().nft_update_attributes(...)`.

---

### Task 5: Testing & Verification

1.  **RustVM Test**: 
    -   Simulate issuance (mock the callback).
    -   Verify `register_agent` actually transfers an NFT to the user.
    -   Verify `update_agent` changes attributes.
2.  **ABI**:
    -   Run `mxpy contract build` and inspect `output/identity-registry.abi.json`.

---

## Verification Commands
```bash
cargo test -p identity-registry
mxpy contract build identity-registry
```
