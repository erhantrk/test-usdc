use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::{
    FungibleToken, FungibleTokenCore, FungibleTokenResolver,
};
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::borsh::BorshSerialize;
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::{
    env, near, require, AccountId, BorshStorageKey, NearToken, PanicOnDefault, PromiseOrValue,
};

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    Token,
    Metadata,
}

/// MOCK stablecoin for testnet only. Has no value.
#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
    owner_id: AccountId,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, total_supply: U128) -> Self {
        require!(!env::state_exists(), "Already initialized");
        let metadata = FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: "Test USD Coin (testnet mock)".to_string(),
            symbol: "tUSDC".to_string(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: 6,
        };
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(StorageKey::Token),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            owner_id: owner_id.clone(),
        };
        this.token.internal_register_account(&owner_id);
        this.token.internal_deposit(&owner_id, total_supply.into());
        near_contract_standards::fungible_token::events::FtMint {
            owner_id: &owner_id,
            amount: total_supply,
            memo: Some("new"),
        }
        .emit();
        this
    }

    /// Testnet faucet: owner-only mint. Receiver must be registered (storage_deposit).
    pub fn mint(&mut self, account_id: AccountId, amount: U128) {
        require!(
            env::predecessor_account_id() == self.owner_id,
            "Only the owner can mint"
        );
        require!(
            self.token.accounts.contains_key(&account_id),
            "Receiver is not registered"
        );
        self.token.internal_deposit(&account_id, amount.into());
        near_contract_standards::fungible_token::events::FtMint {
            owner_id: &account_id,
            amount,
            memo: Some("mint"),
        }
        .emit();
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }
}

#[near]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo)
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenResolver for Contract {
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let (used, _burned) =
            self.token
                .internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
        used.into()
    }
}

#[near]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.token.storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        self.token.storage_withdraw(amount)
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        #[allow(unused_variables)]
        if let Some((account_id, balance)) = self.token.internal_storage_unregister(force) {
            true
        } else {
            false
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.token.storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.token.storage_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    const SUPPLY: u128 = 1_000_000_000_000;

    fn ctx(predecessor: AccountId, deposit: NearToken) -> VMContextBuilder {
        let mut b = VMContextBuilder::new();
        b.current_account_id(accounts(0))
            .signer_account_id(predecessor.clone())
            .predecessor_account_id(predecessor)
            .attached_deposit(deposit);
        b
    }

    fn setup() -> Contract {
        testing_env!(ctx(accounts(1), NearToken::from_near(0)).build());
        Contract::new(accounts(1), U128(SUPPLY))
    }

    fn register(c: &mut Contract, who: AccountId) {
        testing_env!(ctx(who.clone(), NearToken::from_millinear(10)).build());
        c.storage_deposit(Some(who), None);
    }

    #[test]
    fn metadata() {
        let c = setup();
        let m = c.ft_metadata();
        assert_eq!(m.name, "Test USD Coin (testnet mock)");
        assert_eq!(m.symbol, "tUSDC");
        assert_eq!(m.decimals, 6);
        assert_eq!(m.spec, FT_METADATA_SPEC);
        assert_eq!(c.ft_total_supply().0, SUPPLY);
        assert_eq!(c.ft_balance_of(accounts(1)).0, SUPPLY);
    }

    #[test]
    fn owner_can_mint() {
        let mut c = setup();
        register(&mut c, accounts(2));
        testing_env!(ctx(accounts(1), NearToken::from_near(0)).build());
        c.mint(accounts(2), U128(500));
        assert_eq!(c.ft_balance_of(accounts(2)).0, 500);
        assert_eq!(c.ft_total_supply().0, SUPPLY + 500);
    }

    #[test]
    #[should_panic(expected = "Only the owner can mint")]
    fn non_owner_cannot_mint() {
        let mut c = setup();
        register(&mut c, accounts(2));
        testing_env!(ctx(accounts(2), NearToken::from_near(0)).build());
        c.mint(accounts(2), U128(500));
    }

    #[test]
    #[should_panic(expected = "Receiver is not registered")]
    fn mint_unregistered_fails() {
        let mut c = setup();
        testing_env!(ctx(accounts(1), NearToken::from_near(0)).build());
        c.mint(accounts(3), U128(1));
    }

    #[test]
    fn transfer() {
        let mut c = setup();
        register(&mut c, accounts(2));
        testing_env!(ctx(accounts(1), NearToken::from_yoctonear(1)).build());
        c.ft_transfer(accounts(2), U128(300), None);
        assert_eq!(c.ft_balance_of(accounts(2)).0, 300);
        assert_eq!(c.ft_balance_of(accounts(1)).0, SUPPLY - 300);
    }

    #[test]
    fn transfer_call_refunds_unused() {
        let mut c = setup();
        register(&mut c, accounts(2));
        testing_env!(ctx(accounts(1), NearToken::from_yoctonear(1)).build());
        let _ = c.ft_transfer_call(accounts(2), U128(400), None, "".to_string());
        // Receiver has the tokens while the call is in flight.
        assert_eq!(c.ft_balance_of(accounts(2)).0, 400);
        // Receiver's ft_on_transfer returned 150 unused -> resolve refunds 150.
        testing_env!(
            ctx(accounts(0), NearToken::from_near(0)).build(),
            near_sdk::test_vm_config(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![near_sdk::PromiseResult::Successful(
                near_sdk::serde_json::to_vec(&U128(150)).unwrap()
            )]
        );
        let used = c.ft_resolve_transfer(accounts(1), accounts(2), U128(400));
        assert_eq!(used.0, 250);
        assert_eq!(c.ft_balance_of(accounts(2)).0, 250);
        assert_eq!(c.ft_balance_of(accounts(1)).0, SUPPLY - 250);
    }
}
