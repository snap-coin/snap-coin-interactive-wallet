use std::{collections::HashMap, fs::File, io::Write, path::PathBuf, sync::Arc};

use dioxus::prelude::*;
// use home::Home;
use anyhow::Error;
use connection::Connection;
use futures_channel::oneshot;
use home::Home;
use login::Login;
use navigation::NavigationBar;
use snap_coin::{
    api::client::Client,
    crypto::{keys::Private, Hash},
    full_node::{node_state::SharedNodeState, SharedBlockchain},
};
use tokio::sync::Mutex;
use wallet_manager::WalletManager;

use crate::encryption::encrypt_wallets;

// Screens
mod connection;
mod encryption;
mod home;
mod login;
mod navigation;
mod wallet_manager;

// Components
mod authorize;
mod copy_box;
mod node_log;
mod annotate;

pub const LOGO: Asset = asset!("assets/logo.svg");

/// Returns wallet file path
pub fn wallet_path() -> Result<PathBuf, anyhow::Error> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::Error::msg("Could not determine home directory"))?;
    Ok(home.join(".snap-coin-wallet"))
}

pub fn save_wallets(wallets: &HashMap<String, Private>, pin: &str) -> Result<(), Error> {
    let path = wallet_path()?;
    let mut file = File::create(path)?;
    let encrypted =
        encrypt_wallets(wallets, pin).ok_or_else(|| Error::msg("Failed to encrypt wallets"))?;
    file.write_all(&encrypted)?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Routable)]
pub enum Route {
    #[layout(NavigationBar)]
    #[route("/")]
    Login,
    #[route("/connection")]
    Connection,
    #[route("/home")]
    Home,
    #[route("/wallet-manager")]
    WalletManager,
}

#[derive(Clone)]
pub struct NodeHandle {
    node_state: SharedNodeState,
    blockchain: SharedBlockchain,
    log_file: PathBuf,
}

#[derive(Clone)]
pub struct GlobalContext {
    api_client: Option<Arc<Client>>,
    internal_node: Option<NodeHandle>,
    wallets: HashMap<String, Private>, // Name, key
    selected_wallet: String,
    pin: String,
    show_auth: bool,
    auth_tx: Option<Arc<Mutex<Option<oneshot::Sender<bool>>>>>,
}

fn main() {
    Hash::new(b"INIT"); // Get random x init
    dioxus::launch(|| {
        use_context_provider(|| {
            Signal::new(GlobalContext {
                internal_node: None,
                api_client: None,
                wallets: HashMap::new(),
                selected_wallet: "".to_string(),
                pin: "".to_string(),
                show_auth: false,
                auth_tx: None,
            })
        });

        rsx! {
            document::Stylesheet {
                href: asset!("/assets/tailwind.css")
            }
            document::Stylesheet {
                href: asset!("/assets/main.css")
            }
            Router::<Route> {}
        }
    });
}
