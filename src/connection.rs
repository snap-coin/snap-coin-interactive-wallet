use std::{env::home_dir, fs, net::ToSocketAddrs, path::PathBuf, sync::Arc, time::Duration};

use anyhow::anyhow;
use dioxus::prelude::*;
use snap_coin::{
    api::{api_server::Server, client::Client},
    full_node::{auto_peer::start_auto_peer, connect_peer, create_full_node, ibd::ibd_blockchain},
};
use tokio::{net::lookup_host, time::sleep};

use crate::{node_log::NodeLog, GlobalContext, NodeHandle, Route};

fn latest_log_file(node_path: &str) -> Option<PathBuf> {
    let logs_dir = format!("{}/logs", node_path);
    Some(fs::read_dir(&logs_dir).ok()?.next()?.ok()?.path())
}

#[component]
pub fn Connection() -> Element {
    let mut status = use_signal(|| "".to_string());
    let mut in_app_peers_setting =
        use_signal(|| "node.snap-coin.net:8998,node.snap-coin.net:7887".to_string());
    let mut external_api_setting = use_signal(|| "127.0.0.1:3003".to_string());

    let global = use_context::<Signal<GlobalContext>>();
    let navigator = use_navigator();
    let mut started_node = use_signal(|| false);
    let node_port = use_signal(|| "".to_string());

    rsx! {
        div {
            class: "p-5",
            h1 { "Set up a Snap Coin Node" }

            div {
                class: format!("grid w-full divide-x divide-gray-600 {}", if started_node() { "grid-cols-1" } else { "grid-cols-2" }),

                div {
                    class: "p-5 flex flex-col gap-5",
                    h2 { class: "text-xl", "Host node in-wallet" }

                    label { "Default Peers" }
                    input {
                        type: "text",
                        placeholder: "node.snap-coin.net:8998",
                        value: in_app_peers_setting,
                        oninput: move |e| in_app_peers_setting.set(e.value())
                    }

                    button {
                        disabled: started_node,
                        onclick: move |_| {
                            if started_node() {
                                return;
                            }

                            started_node.set(true);

                            let peers = in_app_peers_setting();
                            let mut status = status.clone();
                            let mut global = global.clone();
                            let navigator = navigator.clone();
                            let mut node_port = node_port.clone();

                            // Channel to communicate back to UI safely
                            let (tx, rx) = futures_channel::oneshot::channel();
                            let (synced_tx, synced_rx) = futures_channel::oneshot::channel();
                            let mut tx = Some(tx);

                            // ---- Spawn dedicated node thread ----
                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Builder::new_multi_thread()
                                    .enable_all()
                                    .build()
                                    .unwrap();

                                rt.block_on(async move {
                                    let result: anyhow::Result<()> = async {
                                        let node_path = home_dir().unwrap().join("node-mainnet");
                                        let node_path = node_path.to_str().unwrap().to_string();

                                        if fs::exists(node_path.clone() + "/logs/")? {
                                            fs::remove_dir_all(node_path.clone() + "/logs/")?;
                                            fs::create_dir_all(node_path.clone() + "/logs/")?;
                                        }

                                        let (blockchain, node_state) = create_full_node(&node_path, false);

                                        for peer in peers.split(",") {
                                            let peer = lookup_host(peer)
                                                .await?
                                                .next()
                                                .ok_or(anyhow!("Could not resolve {}", peer))?;

                                            connect_peer(peer, &blockchain, &node_state).await?;
                                        }

                                        let auto_peer = start_auto_peer(node_state.clone(), blockchain.clone(), vec![]);

                                        let log = latest_log_file(&node_path).unwrap_or("".parse().unwrap());

                                        let handle = NodeHandle {
                                            node_state: node_state.clone(),
                                            blockchain: blockchain.clone(),
                                            log_file: log,
                                        };

                                        let port = rand::random::<u16>();
                                        let api_server = Server::new(port as u32, blockchain.clone(), node_state.clone());

                                        // ---- START API SERVER ----
                                        api_server.listen().await?;

                                        // ---- SEND HANDLE EARLY ----
                                        if let Some(tx) = tx.take() {
                                            let _ = tx.send(Ok((handle, port)));
                                        }

                                        // ---- RUN IBD INLINE ----
                                        *node_state.is_syncing.write().await = true;

                                        sleep(Duration::from_secs(2)).await;

                                        let peer = node_state
                                            .connected_peers
                                            .read()
                                            .await
                                            .iter()
                                            .next()
                                            .ok_or(anyhow!("No peers"))?
                                            .1
                                            .clone();

                                        println!("IBD status: {:?}", ibd_blockchain(peer, blockchain, false).await);
                                        let _ = synced_tx.send(());

                                        *node_state.is_syncing.write().await = false;

                                        // ---- KEEP NODE ALIVE ----
                                        auto_peer.await?;

                                        Ok(())
                                    }
                                    .await;

                                    if let Err(e) = result {
                                        if let Some(tx) = tx.take() {
                                            let _ = tx.send(Err(e));
                                        }
                                    }
                                });
                            });

                            // ---- UI SIDE ----
                            spawn(async move {
                                match rx.await {
                                    Ok(Ok((handle, port))) => {
                                        global.write().internal_node = Some(handle);
                                        node_port.set(port.to_string());

                                        let client = loop {
                                            match Client::connect(format!("127.0.0.1:{}", port).parse().unwrap()).await {
                                                Ok(c) => break c,
                                                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(200)).await,
                                            }
                                        };

                                        global.write().api_client = Some(Arc::new(client));
                                    }
                                    Ok(Err(e)) => {
                                        status.set(e.to_string());
                                    }

                                    Err(_) => {
                                        status.set("Node thread closed unexpectedly".to_string());
                                    }
                                }
                                match synced_rx.await {
                                    Ok(()) => {
                                        navigator.push(Route::Home);
                                    },
                                    Err(_) => {}
                                }
                            });
                        },
                        "Start node"
                    }

                    {
                        if started_node() {
                            rsx! { p {
                                b {
                                    "Downloading historical blocks..."
                                }
                                br {}
                                "Snap Coin API port: "
                                b {
                                    {node_port}
                                }
                            } }
                        } else {
                            rsx! {}
                        }
                    }
                    NodeLog { }
                }

                {
                    if !started_node() {
                        rsx! {
                            div {
                                class: "p-5 flex flex-col gap-5",
                                h2 { class: "text-xl", "Connect to a running Node" }

                                label { "Node API address" }

                                input {
                                    type: "text",
                                    placeholder: "127.0.0.1:3003",
                                    value: external_api_setting,
                                    oninput: move |e| external_api_setting.set(e.value())
                                }

                                button {
                                    onclick: move |_| {
                                        let addr = match external_api_setting().to_socket_addrs() {
                                            Ok(mut addrs) => match addrs.next() {
                                                Some(addr) => addr,
                                                None => {
                                                    status.set(format!("Could not resolve {}", external_api_setting()));
                                                    return;
                                                }
                                            },
                                            Err(_) => {
                                                status.set(format!("Could not resolve {}", external_api_setting()));
                                                return;
                                            }
                                        };

                                        let mut status = status.clone();
                                        let mut global = global.clone();
                                        let navigator = navigator.clone();

                                        spawn(async move {
                                            let client = Arc::new(match Client::connect(addr).await {
                                                Ok(c) => c,
                                                Err(e) => {
                                                    status.set(e.to_string());
                                                    return;
                                                }
                                            });

                                            {
                                                let mut g = global.write();
                                                g.api_client = Some(client);
                                            }

                                            navigator.push(Route::Home);
                                        });
                                    },
                                    "Connect"
                                }
                            }
                        }
                    } else {
                        rsx! {}
                    }
                }
            }
        }

        p {
            class: "p-10 text-red-400! font-bold",
            "{status}"
        }
    }
}
