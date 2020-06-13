// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

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
	/// Block number
	pub number: u64,
	/// parent block keccak256
	pub parent_hash: [u8; 32],
	/// hash of post-execution state.
	pub post_state: [u8; 32],
}

impl HeadData {
	pub fn hash(&self) -> [u8; 32] {
		tiny_keccak::keccak256(&self.encode())
	}
}

/// Block data for this parachain.
#[derive(Default, Clone, Encode, Decode)]
pub struct BlockData {}

pub trait ExtBlockData {
	fn merge_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>>;
	fn merge(
		&self,
		#[callback]
		#encode
		data0: Vec<u8>,
		#[callback]
		#encode
		data1: Vec<u8>,
	) -> Vec<u8>;
}

pub trait ExtStatusMessage {
    fn set_status(&mut self, message: String);
    fn get_status(&self, account_id: String) -> Option<String>;
}

#[near_bindgen]
impl BlockData {
    pub fn deploy_status_message(&self, account_id: String, amount: u64) {
        Promise::new(account_id)
            .create_account()
            .transfer(amount as u128)
            .add_full_access_key(env::signer_account_pk())
            .deploy_contract(
                include_bytes!("../status-message-contract/status_message.wasm").to_vec(),
            );
    }

    #[result_serializer(borsh)]
    pub fn merge_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>> {
        if arr.len() <= 1 {
            return PromiseOrValue::Value(arr);
        }
        let pivot = arr.len() / 2;
        let arr0 = arr[..pivot].to_vec();
        let arr1 = arr[pivot..].to_vec();
        let prepaid_gas = env::prepaid_gas();
        let account_id = env::current_account_id();

        ext::merge_sort(arr0, &account_id, 0, prepaid_gas / 4)
            .and(ext::merge_sort(arr1, &account_id, 0, prepaid_gas / 4))
            .then(ext::merge(&account_id, 0, prepaid_gas / 4))
            .into()
    }

    fn internal_merge(&self, arr0: Vec<u8>, arr1: Vec<u8>) -> Vec<u8> {
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

    /// Used for callbacks only. Merges two sorted arrays into one. Panics if it is not called by
    /// the contract itself.
    #[result_serializer(borsh)]
    pub fn merge(
        &self,
        #[callback]
        #[serializer(borsh)]
        data0: Vec<u8>,
        #[callback]
        #[serializer(borsh)]
        data1: Vec<u8>,
    ) -> Vec<u8> {
        env::log(format!("Received {:?} and {:?}", data0, data1).as_bytes());
        assert_eq!(env::current_account_id(), env::predecessor_account_id());
        let result = self.internal_merge(data0, data1);
        env::log(format!("Merged {:?}", result).as_bytes());
        result
    }

    //    /// Alternative implementation of merge that demonstrates usage of callback_vec. Uncomment
    //    /// to use.
    //    pub fn merge(&self, #[callback_vec] #[serializer(borsh)] arrs: &mut Vec<Vec<u8>>) -> Vec<u8> {
    //        assert_eq!(env::current_account_id(), env::predecessor_account_id());
    //        self.internal_merge(arrs.pop().unwrap(), arrs.pop().unwrap())
    //    }

    pub fn simple_call(&mut self, account_id: String, message: String) {
        ext_status_message::set_status(message, &account_id, 0, SINGLE_CALL_GAS);
    }
    pub fn complex_call(&mut self, account_id: String, message: String) -> Promise {
        // 1) call status_message to record a message from the signer.
        // 2) call status_message to retrieve the message of the signer.
        // 3) return that message as its own result.
        // Note, for a contract to simply call another contract (1) is sufficient.
        ext_status_message::set_status(message, &account_id, 0, SINGLE_CALL_GAS).then(
            ext_status_message::get_status(
                env::signer_account_id(),
                &account_id,
                0,
                SINGLE_CALL_GAS,
            ),
        )
    }

    pub fn transfer_money(&mut self, account_id: String, amount: u64) {
        Promise::new(account_id).transfer(amount as u128);
    }
}

/// Start state mismatched with parent header's state hash.
#[derive(Debug)]
pub struct StateMismatch;

/// Execute a block body on top of given parent head, producing new parent head
/// if valid.
pub fn execute(
	parent_hash: [u8; 32],
	parent_head: HeadData,
	block_data: &BlockData,
) -> Result<HeadData, StateMismatch> {
	debug_assert_eq!(parent_hash, parent_head.hash());

	if hash_state(block_data.state) != parent_head.post_state {
		return Err(StateMismatch);
	}

	let new_state = block_data.state.overflowing_add(block_data.add).0;

	Ok(HeadData {
		number: parent_head.number + 1,
		parent_hash,
		post_state: hash_state(new_state),
	})
}
