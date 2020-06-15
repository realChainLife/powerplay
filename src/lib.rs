//! Basic parachain that adds a number as part of its state.

#![no_std]

#![cfg_attr(not(feature = "std"), feature(core_intrinsics, lang_items, core_panic_info, alloc_error_handler))]

use codec::{Encode, Decode};

#[cfg(not(feature = "std"))]
mod wasm_validation;

#[cfg(not(feature = "std"))]
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Head data for this parachain.
#[derive(Default, Clone, Hash, Eq, PartialEq, Encode, Decode)]
pub struct HeadData {
	pub number: u64,
	pub parent_hash: [u8; 32],
	pub post_state: [u8; 32],
}

impl HeadData {
	pub fn hash(&self) -> [u8; 32] {
		tiny_keccak::keccak256(&self.encode())
	}
}

/// Block data for this parachain.
#[derive(Default, Clone, Encode, Decode)]
pub struct BlockData {
	pub state: [u8; 32],
	pub data: CrossChain,
}

/// This is our custom type, to be stored on-chain:
#[derive(Default, Clone, Encode, Decode)]
pub struct CrossChain {}

// Something that can run a cross-chain function:
pub trait BuildCrossChain {
	/// The crosschain context produced by the `build` function.
	type CrossChain: self::CrossChain;
	
	/// Build the `CrossChain`
    fn build_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>>;
    fn build(
        &self,
        #[callback]
        #[encode]
        data0: Vec<u8>,
        #[callback]
        #[encode]
        data1: Vec<u8>,
    ) -> CrossChain Vec<u8>;
}

pub trait ChainStatusMessage {
	fn set_status(&mut self, message: String);
	fn get_status(&self, id: String) -> Option<String>;
}

impl CrossChain {
    pub fn deploy_status_message(&self, id: String, amount: u64) {
        Promise::new(account_id)
            .create_account()
            .transfer(amount as u128)
            .add_full_access_key(env::signer_account_pk())
            .deploy_crosschain();
    }

    #[result_encode]
    pub fn build_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>> {
        if arr.len() <= 1 {
            return PromiseOrValue::Value(arr);
        }
        let pivot = arr.len() / 2;
        let arr0 = arr[..pivot].to_vec();
        let arr1 = arr[pivot..].to_vec();
        let prepaid_gas = env::prepaid_gas();
        let account_id = env::current_account_id();

        ext::build_sort(arr0, &account_id, 0)
            .and(ext::build_sort(arr1, &account_id, 0))
            .then(ext::build(&account_id, 0))
            .into()
    }

    fn internal_build(&self, arr0: Vec<u8>, arr1: Vec<u8>) -> Vec<u8> {
        let mut i = 0usize;
        let mut j = 0usize;
        let mut result = vec![];
        loop {
            if i == arr0.len() {
                result.extend(&arr1[j..]);
                break;
            }
            if j == arr1.len() {
                result.extend(&arr0[i..]);
                break;
            }
            if arr0[i] < arr1[j] {
                result.push(arr0[i]);
                i += 1;
            } else {
                result.push(arr1[j]);
                j += 1;
            }
        }
        result
    }

    #[result_encode]
    pub fn build(
        &self,
        #[callback]
        #[encode]
        data0: Vec<u8>,
        #[callback]
        #[encode]
        data1: Vec<u8>,
    ) -> Vec<u8> {
        env::log(format!("Received {:?} and {:?}", data0, data1).as_bytes());
        assert_eq!(env::current_account_id(), env::predecessor_account_id());
        let result = self.internal_merge(data0, data1);
        env::log(format!("Built {:?}", result).as_bytes());
        result
    }

    pub fn simple_call(&mut self, account_id: String, message: String) {
        ext_status_message::set_status(message, &account_id, 0);
    }
    pub fn complex_call(&mut self, account_id: String, message: String) -> Promise {
        ext_status_message::set_status(message, &account_id, 0).then(
            ext_status_message::get_status(
                env::signer_account_id(),
                &account_id,
                0,
            ),
        )
    }

    pub fn transfer_money(&mut self, account_id: String, amount: u64) {
        Promise::new(account_id).transfer(amount as u128);
    }
}
