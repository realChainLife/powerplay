// Copyright 2018-2020 Parity Technologies (UK) Ltd.
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

//! Collator for polkadot

use std::collections::HashMap;
use std::sync::Arc;

use powerplay::{HeadData as PowerplayHead, BlockData as PowerplayBody};
use sp_core::Pair;
use codec::{Encode, Decode};
use primitives::{
	Hash,
	parachain::{HeadData, BlockData, Id as ParaId, LocalValidationData, GlobalValidationSchedule},
};
use collator::{
	InvalidHead, ParachainContext, Network, BuildParachainContext, Cli, SubstrateCli,
};
use parking_lot::Mutex;
use futures::future::{Ready, ok, err, TryFutureExt};

const GENESIS: PowerplayHead = PowerplayHead {
	number: 0,
	parent_hash: [0; 32],
	post_state: [
		1, 27, 77, 3, 221, 140, 1, 241, 4, 145, 67, 207, 156, 76, 129, 126, 75,
		22, 127, 29, 27, 131, 229, 198, 240, 241, 13, 137, 186, 30, 123, 206
	],
};

const GENESIS_BODY: PowerplayBody = PowerplayBody {
	state: 0,
	add: 0,
};

#[derive(Clone)]
struct PowerplayContext {
	db: Arc<Mutex<HashMap<PowerplayHead, PowerplayBody>>>,
	/// We store it here to make sure that our interfaces require the correct bounds.
	_network: Option<Arc<dyn Network>>,
}

/// The parachain context.
impl ParachainContext for PowerplayContext {
	type ProduceCandidate = Ready<Result<(BlockData, HeadData), InvalidHead>>;

	fn produce_candidate(
		&mut self,
		_relay_parent: Hash,
		_global_validation: GlobalValidationSchedule,
		local_validation: LocalValidationData,
	) -> Self::ProduceCandidate
	{
		let powerplay_head = match PowerplayHead::decode(&mut &local_validation.parent_head.0[..]) {
			Ok(powerplay_head) => powerplay_head,
			Err(_) => return err(InvalidHead)
		};

		let mut db = self.db.lock();

		let last_body = if powerplay_head == GENESIS {
			GENESIS_BODY
		} else {
			db.get(&powerplay_head)
				.expect("All past bodies stored since this is the only collator")
				.clone()
		};

		let next_body = PowerplayBody {
			state: last_body.state.overflowing_add(last_body.add).0,
			add: powerplay_head.number % 100,
		};

		let next_head = powerplay::execute(powerplay_head.hash(), powerplay_head, &next_body)
			.expect("good execution params; qed");

		let encoded_head = HeadData(next_head.encode());
		let encoded_body = BlockData(next_body.encode());

		println!("Created collation for #{}, post-state={}",
			next_head.number, next_body.state.overflowing_add(next_body.add).0);

		db.insert(next_head.clone(), next_body);
		ok((encoded_body, encoded_head))
	}
}

impl BuildParachainContext for PowerplayContext {
	type ParachainContext = Self;

	fn build<Client, SP, Extrinsic>(
		self,
		_: Arc<Client>,
		_: SP,
		network: impl Network + Clone + 'static,
	) -> Result<Self::ParachainContext, ()> {
		Ok(Self { _network: Some(Arc::new(network)), ..self })
	}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let key = Arc::new(Pair::from_seed(&[1; 32]));
	let id: ParaId = 100.into();

	println!("Starting powerplay collator with genesis: ");

	{
		let encoded = GENESIS.encode();
		println!("Dec: {:?}", encoded);
		print!("Hex: 0x");
		for byte in encoded {
			print!("{:02x}", byte);
		}

		println!();
	}

	let context = PowerplayContext {
		db: Arc::new(Mutex::new(HashMap::new())),
		_network: None,
	};

	let cli = Cli::from_iter(&["-dev"]);
	let runner = cli.create_runner(&cli.run.base)?;
	runner.async_run(|config| {
		collator::start_collator(
			context,
			id,
			key,
			config,
			option,
		).map_err(|e| e.into())
	})?;

	Ok(())
}
