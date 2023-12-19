use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{collections::BTreeMap, io::Write, rand::RngCore, vec, vec::Vec, UniformRand};
use digest::Digest;
use dock_crypto_utils::serde_utils::ArkObjectBytes;
use schnorr_pok::{
	compute_random_oracle_challenge, error::SchnorrError, impl_proof_of_knowledge_of_discrete_log,
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub struct ThresholdClient {
	message_queue: Vec<Vec<u8>>,
	prev_shares: Vec<Vec<u8>>,
}

impl ThresholdClient {
	fn new() -> Self {
		Self { message_queue: vec![], current_key: vec![] }
	}

	async fn start() {
		let listener =
			TcpListener::bind("127.0.0.1:8080").await.expect("Failed to bind to address");

		let (tx, _) = mpsc::channel::<String>(32);

		let participants = Arc::new(Mutex::new(HashSet::new()));

		while let Ok((socket, _)) = listener.accept().await {
			let tx = tx.clone();
			let participants = participants.clone();

			tokio::spawn(handle_client(socket, tx, participants));
		}
	}

	async fn broadcast_message(msg: &str) {
		let listener =
			TcpListener::bind("127.0.0.1:8080").await.expect("Failed to bind to address");

		let (tx, _) = mpsc::channel::<String>(32);

		let participants = Arc::new(Mutex::new(HashSet::new()));

		while let Ok((socket, _)) = listener.accept().await {
			let tx = tx.clone();
			let participants = participants.clone();

			let peer_addr = socket.peer_addr().unwrap().to_string();
			println!("{} connected", peer_addr);

			{
				let mut participants = participants.lock().await;
				participants.insert(peer_addr.clone());
				drop(participants);

				let mut participants = participants.lock().await;
				let participant_list: Vec<String> = participants.iter().cloned().collect();
				let _ = tx.send(format!("Participants: {:?}", participant_list)).await;
			}

			let (tx_clone, mut rx) = mpsc::channel::<String>(32);
			tokio::spawn(send_messages(socket.clone(), rx));

			while let Some(message) = tx_clone.recv().await {
				let mut participants = participants.lock().await;
				let participant_list: Vec<String> = participants.iter().cloned().collect();
				let _ = tx.send(format!("Participants: {:?}", participant_list)).await;
				drop(participants);

				if message.trim().to_lowercase() == "exit" {
					println!("{} disconnected", peer_addr);
					break
				}
			}

			let mut participants = participants.lock().await;
			participants.remove(&peer_addr);
		}
	}

	async fn handle_client() {
		let listener =
			TcpListener::bind("127.0.0.1:8080").await.expect("Failed to bind to address");

		let (tx, _) = mpsc::channel::<String>(32);

		let participants = Arc::new(Mutex::new(HashSet::new()));

		while let Ok((socket, _)) = listener.accept().await {
			let peer_addr = socket.peer_addr().unwrap().to_string();
			println!("{} connected", peer_addr);

			while let Some(message) = tx_clone.recv().await {
				if message.trim().to_lowercase() == "exit" {
					println!("{} disconnected", peer_addr);
					break
				}

				self.message_queue.push(message);
			}

			let mut participants = participants.lock().await;
			participants.remove(&peer_addr);
		}
	}
}
