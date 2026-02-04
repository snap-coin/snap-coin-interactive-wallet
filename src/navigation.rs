use dioxus::prelude::*;

use crate::{authorize::Authorize, node_log::NodeLog, GlobalContext, Route, LOGO};

const WALLET_ICON: Asset = asset!("../assets/wallet.svg");

#[component]
pub fn NavigationBar() -> Element {
    let navigator = use_navigator();
    let mut global_context = consume_context::<Signal<GlobalContext>>();

    let mut wallet_drop_open = use_signal(|| false);
    let mut node_log_open = use_signal(|| false);

    rsx! {
        div {
            class: "flex flex-row items-center justify-between border-b border-gray-600 gap-5 p-5 h-20",
            div {
                class: "flex items-center gap-5 cursor-pointer",
                onclick: move |_| {
                    navigator.replace(Route::Home);
                },
                img {
                    class: "h-15",
                    src: LOGO,
                }
                h3 { "Snap Coin Wallet" }
            }
            div {
                class: "relative flex flex-row items-center gap-5",

                {
                    if global_context().internal_node.is_some() {
                        rsx! {
                            button {
                                onclick: move |_| {
                                    node_log_open.set(!node_log_open());
                                },
                                "Node Logs"
                            }
                        }
                    } else { rsx! {} }
                }

                // Wallet button
                div {
                    class: format!("border border-gray-600 p-2 gap-2 flex flex-row items-center rounded-lg hover:cursor-pointer w-40 {} justify-between transition-all", if wallet_drop_open() { "w-100" } else { "" }),
                    onclick: move |_| {
                        wallet_drop_open.set(!wallet_drop_open());
                    },

                    p {
                        class: "truncate m-1 p-1",
                        {
                            let selected_wallet = global_context().selected_wallet;
                            if selected_wallet == "" {
                                "No wallet added".to_string()
                            } else {
                                selected_wallet
                            }
                        }
                    }

                    img {
                        src: WALLET_ICON,
                        class: "invert",
                    }
                }

                // Dropdown
                div {
                    class: format!("absolute top-full right-0 mt-1 z-50 w-40 {} border border-gray-600 bg-[var(--bg)] p-2 flex flex-col rounded-lg shadow-lg transition-all", if wallet_drop_open() { "w-100" } else { "opacity-0 -translate-y-500" }),

                    for wallet in global_context().wallets {
                        {
                            let wallet_name = wallet.0.clone();
                            rsx! {
                                p {
                                    class: "hover:bg-gray-900 p-1 px-2 m-1 rounded-md cursor-pointer truncate",
                                    onclick: move |_| {
                                        global_context.write().selected_wallet = wallet_name.clone();
                                        wallet_drop_open.set(false);
                                    },
                                    { wallet.0.clone() }
                                }
                            }
                        }
                    }
                    hr {
                        class: "text-gray-600 mx-1"
                    }
                    p {
                        class: "hover:bg-gray-900 p-1 px-2 m-1 rounded-md cursor-pointer truncate",
                        onclick: move |_| {
                            navigator.replace(Route::WalletManager);
                            wallet_drop_open.set(false);
                        },
                        "Manage your wallets"
                    }
                }

                NodeLog { class: format!("absolute w-200! right-0 top-0 mt-20 z-49 transition-all {}", if node_log_open() && global_context().internal_node.is_some() { "" } else { "-translate-y-250" }) }
            }

        }
        Outlet::<Route> {}
        Authorize {}
    }
}
