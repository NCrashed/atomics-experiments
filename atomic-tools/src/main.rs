use bitcoin::Network;
use bitcoin::util::bip32::{ExtendedPrivKey, ExtendedPubKey};
use miniscript::policy::{Concrete, Liftable};
use miniscript::{Descriptor, DescriptorTrait};
use std::str::FromStr;
use bitcoin::util::ecdsa::PrivateKey;
use bitcoin::hashes::sha256;
use bitcoin::hashes::Hash;
use bitcoin::hashes::hex::ToHex;
use std::collections::BTreeMap;
use bitcoin::util::bip32::DerivationPath;
use bitcoin::blockdata::transaction::SigHashType;

use bdk::wallet::tx_builder::TxOrdering;
use bdk::wallet::coin_selection::LargestFirstCoinSelection;
use bdk::Wallet;
use bdk::database::MemoryDatabase;
use bdk::blockchain::{noop_progress, ElectrumBlockchain};
use bdk::electrum_client::Client;
use bdk::KeychainKind;

fn main() {
    let secp256k1 = secp256k1::Secp256k1::new();

    let alice_root_private = ExtendedPrivKey::from_str("tprv8ZgxMBicQKsPeAuGznXJZwfWHgWo86dFuufRBZN7ZT44UzoNG2cYmZLNLrnsm7eXhGSeccRU2nTtxunT11UkpqrRhJQefBnFJeHBddF68bg").unwrap();
    let alice_root_public = ExtendedPubKey::from_private(&secp256k1, &alice_root_private);

    let alice_private = alice_root_private.derive_priv(&secp256k1, &DerivationPath::from_str("m/84'/1'/0'/0").unwrap()).unwrap();
    let alice_public =  ExtendedPubKey::from_private(&secp256k1, &alice_private);

    let alice_safe_private = alice_root_private.derive_priv(&secp256k1, &DerivationPath::from_str("m/84'/2'/0'/0").unwrap()).unwrap();
    let alice_safe_public = ExtendedPubKey::from_private(&secp256k1, &alice_safe_private);

    let bob_private = ExtendedPrivKey::from_str("tprv8ZgxMBicQKsPei56wJPNt9u2132Ynncp2qXdfSHszobnyjaGjQwxQBGASUidc1unmEmpyMQ9XzLgvbN36MDW7LNziVFdXVGMrx6ckMHuRmd").unwrap();
    let bob_public = ExtendedPubKey::from_private(&secp256k1, &bob_private);
    let bob_safe_private = bob_private.ckd_priv(&secp256k1, 0.into()).unwrap();
    let bob_safe_public = ExtendedPubKey::from_private(&secp256k1, &bob_safe_private);

    let exchange_secret = "hello";
    let exchange_secret_hash = sha256::Hash::hash(exchange_secret.as_bytes()).into_inner().to_hex();
 
    let alice_raw_descr = format!("or(and(pk({alice}),after({deadline})),and(pk({bob}),sha256({secret_hash})))",
        secret_hash = exchange_secret_hash,
        alice = alice_public.public_key,
        bob = bob_public.public_key,
        deadline = "256"
    );
    let alice_script = Concrete::<bitcoin::PublicKey>::from_str(&alice_raw_descr).unwrap();

    let alice_descriptor = Descriptor::new_wsh(
        alice_script
            .compile()
            .expect("Policy compilation only fails on resource limits or mixed timelocks"),
    )
    .expect("Resource limits");

    assert!(alice_descriptor.sanity_check().is_ok());
    println!("Alice descriptor: {}", alice_descriptor);
    println!("Alice lifted: {}", alice_descriptor.lift().unwrap());
    println!("Alice pubkey script: {}", alice_descriptor.script_pubkey());
    println!("Alice redeem script: {}", alice_descriptor.explicit_script());
    println!("Alice address: {}", alice_descriptor.address(Network::Regtest).unwrap());
    println!("");


    let bob_raw_descr = format!("or(and(pk({bob}),after({deadline})),and(pk({alice}),sha256({secret_hash})))",
        secret_hash = exchange_secret_hash,
        alice = alice_public.public_key,
        bob = bob_private.private_key,
        deadline = "128"
    );
    println!("{}", bob_raw_descr);
    let bob_script = Concrete::<bitcoin::PublicKey>::from_str(&bob_raw_descr).unwrap();


    let bob_descriptor = Descriptor::new_wsh(
        bob_script
            .compile()
            .expect("Policy compilation only fails on resource limits or mixed timelocks"),
    )
    .expect("Resource limits");

    assert!(bob_descriptor.sanity_check().is_ok());
    println!("Bob descriptor: {}", bob_descriptor);
    println!("Bob lifted: {}", bob_descriptor.lift().unwrap());
    println!("Bob pubkey script: {}", bob_descriptor.script_pubkey());
    println!("Bob redeem script: {}", bob_descriptor.explicit_script());
    println!("Bob address: {}", bob_descriptor.address(Network::Regtest).unwrap());
    println!("");

    let bob_safe_descriptor = Concrete::<bitcoin::PublicKey>::from_str(&format!("pk({bob})", bob = bob_safe_public.public_key)).unwrap();
    let bob_safe = Descriptor::new_wsh(
        bob_safe_descriptor
            .compile()
            .expect("Policy compilation only fails on resource limits or mixed timelocks"),
    )
    .expect("Resource limits");

    let client = Client::new("127.0.0.1:51401").unwrap();
    let alice_wallet = Wallet::new(
        &format!("{}", alice_descriptor),
        None,
        bitcoin::Network::Regtest,
        MemoryDatabase::default(),
        ElectrumBlockchain::from(client)
    ).unwrap();

    alice_wallet.sync(noop_progress(), None).unwrap();
    let alice_locked = alice_wallet.get_balance().unwrap();
    println!("Alice locked balance: {} SAT", alice_locked);

    let client = Client::new("127.0.0.1:51401").unwrap();
    let bob_wallet = Wallet::new(
        &format!("{}", bob_descriptor),
        None,
        bitcoin::Network::Regtest,
        MemoryDatabase::default(),
        ElectrumBlockchain::from(client)
    ).unwrap();

    bob_wallet.sync(noop_progress(), None).unwrap();

    println!("Bob locked balance: {} SAT", bob_wallet.get_balance().unwrap());

    if alice_locked > 0 {
        let alice_locked_utxo = &alice_wallet.list_unspent().unwrap()[0];
        let alice_locked_tx = alice_wallet.list_transactions(true).unwrap()[0].transaction.clone().unwrap();
        let mut alice_locked_input: bitcoin::util::psbt::Input = Default::default(); 
        // alice_locked_input.sighash_type = Some(SigHashType::Single);
        alice_locked_input.non_witness_utxo = Some(alice_locked_tx);
        alice_locked_input.witness_utxo = Some(alice_locked_utxo.txout.clone());
        alice_locked_input.redeem_script = Some(alice_descriptor.explicit_script());
        let mut preimage_map = BTreeMap::new();
        preimage_map.insert(sha256::Hash::from_str(&exchange_secret_hash).unwrap(), exchange_secret.as_bytes().to_owned());
        alice_locked_input.sha256_preimages = preimage_map;

        let policies = bob_wallet.policies(KeychainKind::External).unwrap().unwrap();
        let mut policies_path = BTreeMap::new();
        policies_path.insert(policies.id, vec![0]);

        let (mut psbt1, details) = {
            let mut builder = bob_wallet.build_tx(); //.coin_selection(LargestFirstCoinSelection);
            builder
                .ordering(TxOrdering::Untouched)
                .policy_path(policies_path, KeychainKind::External)
                .add_foreign_utxo(alice_locked_utxo.outpoint, alice_locked_input, alice_descriptor.max_satisfaction_weight().unwrap()).unwrap()
                .drain_wallet()
                .set_single_recipient(bob_safe.script_pubkey());
            builder.finish().unwrap()
        };

        // let policies = alice_wallet.policies(KeychainKind::External).unwrap().unwrap();
        // let mut policies_path = BTreeMap::new();
        // policies_path.insert(policies.id, vec![0]);

        // let (mut psbt1, details) = {
        //     let mut builder = alice_wallet.build_tx().coin_selection(LargestFirstCoinSelection);
        //     builder
        //         .ordering(TxOrdering::Untouched)
        //         .policy_path(policies_path, KeychainKind::External)
        //         .drain_wallet()
        //         .set_single_recipient(bob_safe.script_pubkey());
        //     builder.finish().unwrap()
        // };

        bob_wallet.sign(&mut psbt1, Default::default()).unwrap();
        println!("{:?}", psbt1);
        println!("{:?}", details);
        let tx1 = psbt1.extract_tx();
        println!("{:?}", tx1);
        let txid = bob_wallet.broadcast(tx1).unwrap();
        println!("Spending alice funds by bob in {}", txid);
    }
    // let bob_tx = bitcoin::Transaction {
    //     version: 2,
    //     lock_time: 0,
    //     input: vec![bitcoin::TxIn {
    //         previous_output: "77219107469c77825336ba253de9fe96c87a9cad560ad170a8fb38d7c90c443f:1".parse().unwrap(),
    //         script_sig: bitcoin::Script::new(),
    //         sequence: 0xffffffff,
    //         witness: vec![],
    //     }],
    //     output: vec![bitcoin::TxOut {
    //         script_pubkey: bob_safe.script_pubkey(),
    //         value: 10_000,
    //     }],
    // };
    // let mut bob_psbt = bitcoin::util::psbt::PartiallySignedTransaction::from_unsigned_tx(bob_tx).unwrap();
    // let mut preimage_map = BTreeMap::new();
    // preimage_map.insert(sha256::Hash::from_str(&exchange_secret_hash).unwrap(), exchange_secret.as_bytes().to_owned());
    // bob_psbt.inputs[0].sha256_preimages = preimage_map;
    // bob_psbt.inputs[0].redeem_script = Some(alice_descriptor.explicit_script());
}