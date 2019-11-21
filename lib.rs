#![feature(proc_macro_hygiene)]
#![cfg_attr(not(feature = "std"), no_std)]

use ink_core::memory::string::String;
use ink_core::storage;
use ink_lang2 as ink;

#[cfg(feature = "ink-as-dependency")]
pub use crate::mintable::Mintable;

#[ink::contract(version = "0.1.0")]
mod mintable {

    #[ink(storage)]
    struct Mintable {
        name: storage::Value<String>,
        minter: storage::Value<AccountId>,
        total_supply: storage::Value<Balance>,
        balances: storage::HashMap<AccountId, Balance>,
        allowances: storage::HashMap<(AccountId, AccountId), Balance>,
    }

    #[ink(event)]
    struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        value: Balance,
    }

    #[ink(event)]
    struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        #[ink(topic)]
        value: Balance,
    }

    impl Mintable {
        // mintable and burnable erc20 token
        // only minter can mint, but anyone can burn their own token
        #[ink(constructor)]
        fn new(&mut self, name: String) {
            let caller = self.env().caller();
            let initial_supply = 0;

            self.name.set(name);
            self.minter.set(caller);
            self.total_supply.set(initial_supply);
            self.balances.insert(caller, initial_supply);

            self.env().emit_event(Transfer {
                from: None,
                to: Some(caller),
                value: initial_supply,
            });
        }

        // Read
        #[ink(message)]
        fn name(&self) -> String {
            self.name.clone()
        }

        #[ink(message)]
        fn minter(&self) -> AccountId {
            *self.minter
        }

        #[ink(message)]
        fn total_supply(&self) -> Balance {
            *self.total_supply
        }

        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> Balance {
            self.balance_of_or_zero(&owner)
        }

        #[ink(message)]
        fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.allowance_of_or_zero(&owner, &spender)
        }

        // Write
        #[ink(message)]
        fn mint(&mut self, to: AccountId, value: Balance) -> bool {
            self._mint(to, value);
            true
        }

        #[ink(message)]
        fn burn(&mut self, value: Balance) -> bool {
            let from = self.env().caller();
            self._burn(from, value);
            true
        }

        #[ink(message)]
        fn transfer(&mut self, to: AccountId, value: Balance) -> bool {
            let from = self.env().caller();
            self._transfer(from, to, value);
            true
        }

        #[ink(message)]
        fn approve(&mut self, spender: AccountId, value: Balance) -> bool {
            let owner = self.env().caller();
            self._approve(owner, spender, value);
            true
        }

        #[ink(message)]
        fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance) -> bool {
            self._transfer(from, to, value);

            let spender = self.env().caller();
            let allowance = self.allowance_of_or_zero(&from, &spender);
            assert!(allowance >= value);

            self._approve(from, spender, allowance - value);
            true
        }

        // pure rust below
        fn _mint(&mut self, to: AccountId, value: Balance) {
            let caller = self.env().caller();
            assert_eq!(caller, *self.minter);

            let to_balance = self.balance_of_or_zero(&to);
            self.balances.insert(to.clone(), to_balance + value);

            let new_supply = *self.total_supply + value;
            self.total_supply.set(new_supply);

            self.env().emit_event(Transfer {
                from: None,
                to: Some(to),
                value,
            });
        }

        fn _burn(&mut self, from: AccountId, value: Balance) {
            let from_balance = self.balance_of_or_zero(&from);
            assert!(from_balance >= value, "no enough balance to burn");
            self.balances.insert(from.clone(), from_balance - value);

            let new_supply = *self.total_supply - value;
            self.total_supply.set(new_supply);

            self.env().emit_event(Transfer {
                from: Some(from),
                to: None,
                value,
            });
        }

        fn _transfer(&mut self, from: AccountId, to: AccountId, value: Balance) {
            let from_balance = self.balance_of_or_zero(&from);
            assert!(from_balance >= value, "no enough balance to transfer");
            self.balances.insert(from.clone(), from_balance - value);

            let to_balance = self.balance_of_or_zero(&to);
            self.balances.insert(to.clone(), to_balance + value);

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value,
            });
        }

        fn _approve(&mut self, owner: AccountId, spender: AccountId, value: Balance) {
            self.allowances.insert((owner, spender), value);

            self.env().emit_event(Approval {
                owner,
                spender,
                value,
            });
        }

        fn balance_of_or_zero(&self, owner: &AccountId) -> Balance {
            *self.balances.get(owner).unwrap_or(&0)
        }

        fn allowance_of_or_zero(&self, owner: &AccountId, spender: &AccountId) -> Balance {
            *self.allowances.get(&(*owner, *spender)).unwrap_or(&0)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[should_panic(expected = "no enough balance to burn")]
        #[test]
        fn burn_twice_should_panic() {
            let mut mintable = Mintable::new(String::from("Test"));
            let value = 1000;
            mintable.mint(AccountId::default(), value);
            mintable.burn(value * 2);
        }

        #[test]
        fn it_works() {
            let name = String::from("BTC");
            let mut mintable = Mintable::new(name.clone());
            assert_eq!(mintable.name(), name);

            let account = AccountId::from([1u8; 32]);
            // assert_eq!(mintable.mint(account, 1), true);
            // env::test::set_caller::<Types>(account);
            // assert_eq!(mintable.burn(1), true);

            let minter = mintable.minter();
            assert_eq!(minter, AccountId::default());
            let value = 1000;
            assert_eq!(mintable.mint(minter, value), true);
            assert_eq!(mintable.burn(value), true);
            assert_eq!(mintable.total_supply(), 0);

            assert_eq!(mintable.mint(minter, value), true);

            assert_eq!(mintable.balance_of(minter), value);
            assert_eq!(mintable.total_supply(), value);

            assert_eq!(mintable.balance_of(minter), value);
            assert_eq!(mintable.total_supply(), value);

            assert_eq!(mintable.transfer(minter, value), true);
            assert_eq!(mintable.balance_of(minter), value);

            assert_eq!(mintable.transfer(account, value), true);
            assert_eq!(mintable.balance_of(minter), 0);
            assert_eq!(mintable.balance_of(account), value);
        }
    }
}
