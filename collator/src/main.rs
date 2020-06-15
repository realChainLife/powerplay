//! Collator for powerplay

use std::collections::HashMap;
use std::sync::Arc;

use powerplay::{HeadData as PowerplayHead, BlockData as PowerplayBody}, CrossChain;
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
};

const GENESIS_BODY: PowerplayBody = PowerplayBody, CrossChain {
	state: 0,
	add: 0,
	data: crosschain,
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

		let next_body = PowerplayBody, CrossChain {
			state: last_body.state.overflowing_add(last_body.add).0,
			add: powerplay_head.number % 100,
			data: transfer_money(&mut self, account_id: String, amount: u64)
		};

		let next_head = powerplay::execute(powerplay_head.hash(), powerplay_head, &next_body)
			.expect("good execution params; qed");

		let encoded_head = HeadData(next_head.encode());
		let encoded_body = BlockData(next_body.encode());
		let encoded_body = CrossChain(next.body.encode());

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
