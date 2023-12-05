use bitcoin::Amount;
use bitcoincore_rpc::{
	bitcoincore_rpc_json::HashOrHeight,
	json::{ScanTxOutRequest, TxOutSetHashType, Utxo},
	Auth, Client, RawTx, RpcApi,
};

pub async fn fetch_utxos(addr: String) -> Result<Vec<Utxo>, Box<dyn std::error::Error>> {
	let rpc = Client::new(
		"http://localhost:18444",
		Auth::UserPass("bitcoin".to_string(), "talk".to_string()),
	)
	.unwrap();
	let best_block_hash = rpc.get_best_block_hash().unwrap();
	println!("best block hash: {}", best_block_hash);

	let scan_utxos_output =
		rpc.scan_tx_out_set_blocking(&[ScanTxOutRequest::Single(format!("addr({})", addr))]);

	let utxos = scan_utxos_output.unwrap().unspents;
	println!("Fetched utxos {:?}", utxos);

	Ok(utxos)
}

pub fn filter_utxos(
	list: Vec<Utxo>,
	amount: Amount,
) -> Result<Vec<Utxo>, Box<dyn std::error::Error>> {
	// order by the latest
	let mut ordered_list = list.clone();
	ordered_list.sort_by(|a, b| b.height.cmp(&a.height));

	// simple logic, lets pack utxos until we get to the required amount
	let mut filtered_utxos = vec![];
	let mut total_amount_in_utxos = 0;
	for utxo in ordered_list {
		if total_amount_in_utxos >= amount {
			break
		}

		total_amount_in_utxos += utxo.amount;
		filtered_utxos.push(utxo);
	}

	Ok(filtered_utxos)
}

pub fn create_transaction(
	list: Vec<Utxo>,
	recipient: Address,
	amount: Amount,
) -> Result<String, Box<dyn std::error::Error>> {
	let rpc = Client::new(
		"http://localhost:18444",
		Auth::UserPass("bitcoin".to_string(), "talk".to_string()),
	)
	.unwrap();

	let mut inputs = vec![];
	for utxo in inputs {
		inputs.push(json::CreateRawTransactionInput {
			txid: unspent.txid,
			vout: unspent.vout,
			sequence: None,
		})
	}

	let mut output = HashMap::new();
	output.insert(recipient.to_string(), amount);

	let tx = rpc.create_raw_transaction(&[input.clone()], &output, None, Some(true)).unwrap();

	let mempool_accept = rpc.test_mempool_accept(vec![tx].into()).unwrap();

	let estimated_fees = mempool_accept.first().unwrap().fees.unwrap().base;

	println!("estimated_fees {:?}", estimated_fees);

	Ok(tx.raw_hex())
}

pub async fn broadcast_transaction(addr: String) -> Result<Vec<Utxo>, Box<dyn std::error::Error>> {
	let rpc = Client::new(
		"http://localhost:18444",
		Auth::UserPass("bitcoin".to_string(), "talk".to_string()),
	)
	.unwrap();
	let best_block_hash = rpc.get_best_block_hash().unwrap();
	println!("best block hash: {}", best_block_hash);

	let scan_utxos_output =
		rpc.scan_tx_out_set_blocking(&[ScanTxOutRequest::Single(format!("addr({})", addr))]);

	let utxos = scan_utxos_output.unwrap().unspents;
	println!("Fetched utxos {:?}", utxos);

	Ok(utxos)
}



#[derive(Debug, serde::Deserialize)]
struct FeeResponse {
    fastestFee: f64,
    halfHourFee: f64,
    hourFee: f64,
}