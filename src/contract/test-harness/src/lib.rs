/* This file is part of DarkFi (https://dark.fi)
 *
 * Copyright (C) 2020-2023 Dyne.org foundation
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::collections::HashMap;

use darkfi::{
    consensus::{
        ValidatorState, ValidatorStatePtr, TESTNET_BOOTSTRAP_TIMESTAMP, TESTNET_GENESIS_HASH_BYTES,
        TESTNET_GENESIS_TIMESTAMP, TESTNET_INITIAL_DISTRIBUTION,
    },
    runtime::vm_runtime::SMART_CONTRACT_ZKAS_DB_NAME,
    wallet::{WalletDb, WalletPtr},
    zk::{empty_witnesses, ProvingKey, ZkCircuit},
    zkas::ZkBinary,
    Result,
};
use darkfi_dao_contract::{
    DAO_CONTRACT_ZKAS_DAO_EXEC_NS, DAO_CONTRACT_ZKAS_DAO_MINT_NS,
    DAO_CONTRACT_ZKAS_DAO_PROPOSE_BURN_NS, DAO_CONTRACT_ZKAS_DAO_PROPOSE_MAIN_NS,
    DAO_CONTRACT_ZKAS_DAO_VOTE_BURN_NS, DAO_CONTRACT_ZKAS_DAO_VOTE_MAIN_NS,
};
use darkfi_money_contract::{
    client::{MoneyNote, OwnCoin},
    model::Output,
    CONSENSUS_CONTRACT_ZKAS_BURN_NS_V1, CONSENSUS_CONTRACT_ZKAS_MINT_NS_V1,
    CONSENSUS_CONTRACT_ZKAS_PROPOSAL_NS_V1, MONEY_CONTRACT_ZKAS_BURN_NS_V1,
    MONEY_CONTRACT_ZKAS_MINT_NS_V1, MONEY_CONTRACT_ZKAS_TOKEN_FRZ_NS_V1,
    MONEY_CONTRACT_ZKAS_TOKEN_MINT_NS_V1,
};
use darkfi_sdk::crypto::{
    poseidon_hash, Keypair, MerkleTree, Nullifier, PublicKey, SecretKey, CONSENSUS_CONTRACT_ID,
    DAO_CONTRACT_ID, MONEY_CONTRACT_ID,
};
use darkfi_serial::{deserialize, serialize};
use log::info;
use rand::rngs::OsRng;

mod benchmarks;
use benchmarks::TxActionBenchmarks;
mod vks;

mod money_airdrop;
mod money_token;

pub fn init_logger() {
    let mut cfg = simplelog::ConfigBuilder::new();
    cfg.add_filter_ignore("sled".to_string());

    if let Err(_) = simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        //simplelog::LevelFilter::Debug,
        //simplelog::LevelFilter::Trace,
        cfg.build(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    ) {
        return
    }
}

/// Enum representing configured wallet holders
#[derive(Debug, Eq, Hash, PartialEq)]
pub enum Holder {
    Faucet,
    Alice,
    Bob,
    Charlie,
    Rachel,
}

/// Enum representing transaction actions
#[derive(Debug, Eq, Hash, PartialEq)]
pub enum TxAction {
    MoneyAirdrop,
    MoneyTokenMint,
    MoneyTokenFreeze,
    MoneyGenesisMint,
    MoneyTransfer,
    MoneyOtcSwap,
}

pub struct Wallet {
    pub keypair: Keypair,
    pub token_mint_authority: Keypair,
    pub state: ValidatorStatePtr,
    pub money_merkle_tree: MerkleTree,
    pub consensus_staked_merkle_tree: MerkleTree,
    pub consensus_unstaked_merkle_tree: MerkleTree,
    pub wallet: WalletPtr,
    pub unspent_money_coins: Vec<OwnCoin>,
    pub spent_money_coins: Vec<OwnCoin>,
}

impl Wallet {
    pub async fn new(keypair: Keypair, faucet_pubkeys: &[PublicKey]) -> Result<Self> {
        let wallet = WalletDb::new("sqlite::memory:", "foo").await?;
        let sled_db = sled::Config::new().temporary(true).open()?;

        // Use pregenerated vks
        vks::inject(&sled_db)?;

        let state = ValidatorState::new(
            &sled_db,
            *TESTNET_BOOTSTRAP_TIMESTAMP,
            *TESTNET_GENESIS_TIMESTAMP,
            *TESTNET_GENESIS_HASH_BYTES,
            *TESTNET_INITIAL_DISTRIBUTION,
            wallet.clone(),
            faucet_pubkeys.to_vec(),
            false,
            false,
        )
        .await?;

        // Create necessary Merkle trees for tracking
        let money_merkle_tree = MerkleTree::new(100);
        let consensus_staked_merkle_tree = MerkleTree::new(100);
        let consensus_unstaked_merkle_tree = MerkleTree::new(100);

        let unspent_money_coins = vec![];
        let spent_money_coins = vec![];

        let token_mint_authority = Keypair::random(&mut OsRng);

        Ok(Self {
            keypair,
            token_mint_authority,
            state,
            money_merkle_tree,
            consensus_staked_merkle_tree,
            consensus_unstaked_merkle_tree,
            wallet,
            unspent_money_coins,
            spent_money_coins,
        })
    }
}

pub struct TestHarness {
    pub holders: HashMap<Holder, Wallet>,
    pub proving_keys: HashMap<&'static str, (ProvingKey, ZkBinary)>,
    pub tx_action_benchmarks: HashMap<TxAction, TxActionBenchmarks>,
}

impl TestHarness {
    pub async fn new(contracts: &[String]) -> Result<Self> {
        let mut holders = HashMap::new();

        let faucet_kp = Keypair::random(&mut OsRng);
        let faucet_pubkeys = vec![faucet_kp.public];
        let faucet = Wallet::new(faucet_kp, &faucet_pubkeys).await?;
        holders.insert(Holder::Faucet, faucet);

        let alice_kp = Keypair::random(&mut OsRng);
        let alice = Wallet::new(alice_kp, &faucet_pubkeys).await?;
        // Alice is inserted at end of function

        let bob_kp = Keypair::random(&mut OsRng);
        let bob = Wallet::new(bob_kp, &faucet_pubkeys).await?;
        holders.insert(Holder::Bob, bob);

        let charlie_kp = Keypair::random(&mut OsRng);
        let charlie = Wallet::new(charlie_kp, &faucet_pubkeys).await?;
        holders.insert(Holder::Charlie, charlie);

        let rachel_kp = Keypair::random(&mut OsRng);
        let rachel = Wallet::new(rachel_kp, &faucet_pubkeys).await?;
        holders.insert(Holder::Rachel, rachel);

        // Get the zkas circuits and build proving keys
        let mut proving_keys = HashMap::new();
        let alice_sled = alice.state.read().await.blockchain.sled_db.clone();

        macro_rules! mkpk {
            ($db:expr, $ns:expr) => {
                info!("Building ProvingKey for {}", $ns);
                let zkas_bytes = $db.get(&serialize(&$ns))?.unwrap();
                let (zkbin, _): (Vec<u8>, Vec<u8>) = deserialize(&zkas_bytes)?;
                let zkbin = ZkBinary::decode(&zkbin)?;
                let witnesses = empty_witnesses(&zkbin);
                let circuit = ZkCircuit::new(witnesses, zkbin.clone());
                let pk = ProvingKey::build(13, &circuit);
                proving_keys.insert($ns, (pk, zkbin));
            };
        }

        if contracts.contains(&"money".to_string()) {
            let db_handle = alice.state.read().await.blockchain.contracts.lookup(
                &alice_sled,
                &MONEY_CONTRACT_ID,
                SMART_CONTRACT_ZKAS_DB_NAME,
            )?;
            mkpk!(db_handle, MONEY_CONTRACT_ZKAS_MINT_NS_V1);
            mkpk!(db_handle, MONEY_CONTRACT_ZKAS_BURN_NS_V1);
            mkpk!(db_handle, MONEY_CONTRACT_ZKAS_TOKEN_MINT_NS_V1);
            mkpk!(db_handle, MONEY_CONTRACT_ZKAS_TOKEN_FRZ_NS_V1);
        }

        if contracts.contains(&"consensus".to_string()) {
            let db_handle = alice.state.read().await.blockchain.contracts.lookup(
                &alice_sled,
                &CONSENSUS_CONTRACT_ID,
                SMART_CONTRACT_ZKAS_DB_NAME,
            )?;
            mkpk!(db_handle, CONSENSUS_CONTRACT_ZKAS_MINT_NS_V1);
            mkpk!(db_handle, CONSENSUS_CONTRACT_ZKAS_BURN_NS_V1);
            mkpk!(db_handle, CONSENSUS_CONTRACT_ZKAS_PROPOSAL_NS_V1);
        }

        if contracts.contains(&"dao".to_string()) {
            let db_handle = alice.state.read().await.blockchain.contracts.lookup(
                &alice_sled,
                &DAO_CONTRACT_ID,
                SMART_CONTRACT_ZKAS_DB_NAME,
            )?;
            mkpk!(db_handle, DAO_CONTRACT_ZKAS_DAO_EXEC_NS);
            mkpk!(db_handle, DAO_CONTRACT_ZKAS_DAO_MINT_NS);
            mkpk!(db_handle, DAO_CONTRACT_ZKAS_DAO_VOTE_BURN_NS);
            mkpk!(db_handle, DAO_CONTRACT_ZKAS_DAO_VOTE_MAIN_NS);
            mkpk!(db_handle, DAO_CONTRACT_ZKAS_DAO_PROPOSE_BURN_NS);
            mkpk!(db_handle, DAO_CONTRACT_ZKAS_DAO_PROPOSE_MAIN_NS);
        }

        // Build benchmarks map
        let mut tx_action_benchmarks = HashMap::new();
        tx_action_benchmarks.insert(TxAction::MoneyAirdrop, TxActionBenchmarks::new());
        tx_action_benchmarks.insert(TxAction::MoneyTokenMint, TxActionBenchmarks::new());
        tx_action_benchmarks.insert(TxAction::MoneyTokenFreeze, TxActionBenchmarks::new());
        tx_action_benchmarks.insert(TxAction::MoneyGenesisMint, TxActionBenchmarks::new());
        tx_action_benchmarks.insert(TxAction::MoneyOtcSwap, TxActionBenchmarks::new());
        tx_action_benchmarks.insert(TxAction::MoneyTransfer, TxActionBenchmarks::new());

        // Alice jumps down the rabbit hole
        holders.insert(Holder::Alice, alice);

        Ok(Self { holders, proving_keys, tx_action_benchmarks })
    }

    pub fn statistics(&self) {
        info!("==================== Statistics ====================");
        for (action, tx_action_benchmark) in &self.tx_action_benchmarks {
            tx_action_benchmark.statistics(action);
        }
        info!("====================================================");
    }

    pub fn gather_owncoin(
        &mut self,
        holder: Holder,
        output: Output,
        secret_key: Option<SecretKey>,
    ) -> Result<OwnCoin> {
        let wallet = self.holders.get_mut(&holder).unwrap();
        let leaf_position = wallet.money_merkle_tree.mark().unwrap();
        let secret_key = match secret_key {
            Some(key) => key,
            None => wallet.keypair.secret,
        };

        let note: MoneyNote = output.note.decrypt(&secret_key)?;
        let oc = OwnCoin {
            coin: output.coin,
            note: note.clone(),
            secret: secret_key,
            nullifier: Nullifier::from(poseidon_hash([wallet.keypair.secret.inner(), note.serial])),
            leaf_position,
        };

        wallet.unspent_money_coins.push(oc.clone());

        Ok(oc)
    }
}