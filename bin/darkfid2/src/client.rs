use lazy_init::Lazy;
use log::info;

use darkfi::{
    crypto::{address::Address, keypair::Keypair, proof::ProvingKey},
    wallet::walletdb::WalletPtr,
    Result,
};

pub struct Client {
    main_keypair: Keypair,
    wallet: WalletPtr,
    mint_pk: Lazy<ProvingKey>,
    burn_pk: Lazy<ProvingKey>,
}

impl Client {
    pub async fn new(wallet: WalletPtr) -> Result<Self> {
        // Initialize or load the wallet
        wallet.init_db().await?;

        // Check if there is a default keypair and generate one in
        // case we don't have any.
        if wallet.get_default_keypair().await.is_err() {
            // TODO: Clean this up with Option<T> to have less calls.
            if wallet.get_keypairs().await?.is_empty() {
                wallet.keygen().await?;
            }

            wallet.set_default_keypair(&wallet.get_keypairs().await?[0].public).await?;
        }

        // Generate Merkle Tree if we don't have one.
        // if wallet.get_tree().await.is_err() {
        // wallet.tree_gen().await?;
        // }

        let main_keypair = wallet.get_default_keypair().await?;
        info!(target: "CLIENT", "Main keypair: {}", Address::from(main_keypair.public));

        Ok(Self { main_keypair, wallet, mint_pk: Lazy::new(), burn_pk: Lazy::new() })
    }

    pub async fn keygen(&self) -> Result<Address> {
        let kp = self.wallet.keygen().await?;
        Ok(Address::from(kp.public))
    }

    pub async fn get_keypairs(&self) -> Result<Vec<Keypair>> {
        self.wallet.get_keypairs().await
    }
}
