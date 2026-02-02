#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(
    TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode, Clone, PartialEq, Debug,
)]
pub struct AgentDetails<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub uri: ManagedBuffer<M>,
    pub public_key: ManagedBuffer<M>,
    pub owner: ManagedAddress<M>,
}

#[type_abi]
#[derive(
    TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode, Clone, PartialEq, Debug,
)]
pub struct AgentRegisteredEventData<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub uri: ManagedBuffer<M>,
}

#[multiversx_sc::contract]
pub trait IdentityRegistry:
    multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[init]
    fn init(&self) {}

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issue_token)]
    fn issue_token(&self, token_display_name: ManagedBuffer, token_ticker: ManagedBuffer) {
        require!(self.agent_token_id().is_empty(), "Token already issued");
        let issue_cost = self.call_value().egld().clone_value();

        self.agent_token_id().issue_and_set_all_roles(
            EsdtTokenType::NonFungible,
            issue_cost,
            token_display_name,
            token_ticker,
            0,
            None,
        );
    }

    #[endpoint(register_agent)]
    fn register_agent(&self, name: ManagedBuffer, uri: ManagedBuffer, public_key: ManagedBuffer) {
        require!(!self.agent_token_id().is_empty(), "Token not issued");

        let caller = self.blockchain().get_caller();
        let nonce = self.agent_token_nonce().update(|n| {
            *n += 1;
            *n
        });

        let details = AgentDetails {
            name: name.clone(),
            uri: uri.clone(),
            public_key: public_key.clone(),
            owner: caller.clone(),
        };

        // Mint Soulbound NFT
        // Roles required: ESDTRoleNFTCreate, ESDTRoleNFTUpdateAttributes
        // Transfer role should be kept by the contract to ensure souldbound property
        self.send().esdt_nft_create(
            &self.agent_token_id().get_token_id(),
            &BigUint::from(1u64),
            &name,
            &BigUint::from(0u64),  // No royalties
            &ManagedBuffer::new(), // Attributes hash (optional)
            &details,
            &self.create_uris_vec(uri.clone()),
        );

        // Send NFT to caller
        self.tx()
            .to(&caller)
            .single_esdt(
                &self.agent_token_id().get_token_id(),
                nonce,
                &BigUint::from(1u64),
            )
            .transfer();

        self.agent_registered_event(&caller, nonce, AgentRegisteredEventData { name, uri });
    }

    #[endpoint(update_agent)]
    fn update_agent(&self, nonce: u64, new_uri: ManagedBuffer, new_public_key: ManagedBuffer) {
        require!(!self.agent_token_id().is_empty(), "Token not issued");

        let caller = self.blockchain().get_caller();
        let token_id = self.agent_token_id().get_token_id();

        // Fetch attributes to check ownership and preserve data
        let mut details: AgentDetails<Self::Api> =
            self.blockchain().get_token_attributes(&token_id, nonce);
        require!(caller == details.owner, "Only owner can update agent");

        // Update URI and PK
        details.uri = new_uri.clone();
        details.public_key = new_public_key;

        self.send()
            .nft_update_attributes(&token_id, nonce, &details);

        self.agent_updated_event(nonce, &new_uri);
    }

    fn create_uris_vec(&self, uri: ManagedBuffer) -> ManagedVec<ManagedBuffer> {
        let mut uris = ManagedVec::new();
        uris.push(uri);
        uris
    }

    #[view(getAgent)]
    fn get_agent(&self, nonce: u64) -> AgentDetails<Self::Api> {
        let token_id = self.agent_token_id().get_token_id();
        self.blockchain().get_token_attributes(&token_id, nonce)
    }

    // Events

    #[event("agentRegistered")]
    fn agent_registered_event(
        &self,
        #[indexed] owner: &ManagedAddress,
        #[indexed] nonce: u64,
        data: AgentRegisteredEventData<Self::Api>,
    );

    #[event("agentUpdated")]
    fn agent_updated_event(&self, #[indexed] nonce: u64, uri: &ManagedBuffer);

    // Storage Mappers

    #[view(getAgentTokenId)]
    #[storage_mapper("agentTokenId")]
    fn agent_token_id(&self) -> NonFungibleTokenMapper;

    #[view(agent_token_nonce)]
    #[storage_mapper("agentTokenNonce")]
    fn agent_token_nonce(&self) -> SingleValueMapper<u64>;
}
