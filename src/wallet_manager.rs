use crate::{authorize::ask_for_auth, copy_box::CopyBox, save_wallets, GlobalContext};
use dioxus::prelude::*;
use snap_coin::crypto::keys::Private;
use tokio::time::{sleep, Duration};

const WRITE_ICON: Asset = asset!("../assets/write.svg");
const PRIVATE_ICON: Asset = asset!("../assets/private.svg");
const TRASH_ICON: Asset = asset!("../assets/trash.svg");

#[component]
pub fn WalletManager() -> Element {
    let mut global_state = consume_context::<Signal<GlobalContext>>();
    let mut editing_wallet = use_signal(|| None::<String>);
    let mut showing_private = use_signal(|| None::<String>);
    let mut edit_value = use_signal(String::new);

    let mut creating_wallet = use_signal(|| false);
    let mut new_wallet_name = use_signal(|| "".to_string());
    let mut new_wallet_private = use_signal(|| "".to_string());
    let mut new_wallet_error = use_signal(|| "".to_string());

    // ✅ added
    let mut show_backup_popup = use_signal(|| false);
    let mut backup_countdown = use_signal(|| 10i32);

    rsx! {
        div {
            class: "p-5",
            h1 { "Wallet Manager" }

            // ✅ BACKUP POPUP
            if show_backup_popup() {
                div {
                    class: "fixed inset-0 bg-black/70 flex items-center justify-center z-50",

                    div {
                        class: "bg-[var(--panel-soft)] p-6 rounded-xl max-w-md w-full flex flex-col gap-4",

                        h2 { "Backup Your Private Key" }

                        p {
                            "This private key gives full access to your wallet.
                            If you lose it, your funds are permanently lost."
                        }

                        p { "Save it somewhere secure, offline, before continuing." }

                        div {
                            class: "font-mono p-3 bg-black rounded break-all border",
                            "{new_wallet_private()}"
                        }

                        button {
                            disabled: backup_countdown() > 0,
                            class: "w-full",

                            onclick: move |_| {
                                let private = Private::new_from_base36(&new_wallet_private()).unwrap();

                                global_state.write().wallets.insert(new_wallet_name(), private);
                                save_wallets(&global_state().wallets, &global_state().pin).unwrap();

                                new_wallet_name.set("".to_string());
                                new_wallet_private.set("".to_string());
                                creating_wallet.set(false);
                                show_backup_popup.set(false);
                            },

                            {
                                if backup_countdown() > 0 {
                                    format!("I saved it ({})", backup_countdown())
                                } else {
                                    "I saved it".to_string()
                                }
                            }
                        }
                    }
                }
            }

            div {
                class: "flex flex-col items-center w-full p-5",
                div {
                    class: "flex flex-col max-w-500",
                    h3 { "Your wallets" }

                    div {
                        class: "divide-y divide-gray-600",

                        for (wallet, private) in global_state().wallets.iter() {
                            {
                                let wallet = wallet.clone();
                                let wallet_copy = wallet.clone();
                                let wallet_copy2 = wallet.clone();
                                let public = private.to_public();

                                rsx! {
                                    p {
                                        key: "{wallet}",
                                        class: "w-full p-5 grid grid-cols-[max-content_1fr] items-center gap-5",

                                        if editing_wallet() == Some(wallet.clone()) {
                                            input {
                                                class: "border p-1 rounded",
                                                value: "{edit_value()}",

                                                oninput: move |e| {
                                                    edit_value.set(e.value());
                                                },

                                                onkeydown: move |e| {
                                                    if e.key() == Key::Enter {
                                                        let mut new_name = edit_value();
                                                        new_name = new_name.trim().to_string();

                                                        let can_rename = global_state.with(|g| {
                                                            !new_name.is_empty()
                                                                && (!g.wallets.contains_key(&new_name) || new_name == wallet)
                                                        });

                                                        if can_rename {
                                                            global_state.with_mut(|g| {
                                                                if wallet == g.selected_wallet {
                                                                    g.selected_wallet = new_name.clone();
                                                                }
                                                                if let Some(v) = g.wallets.remove(&wallet) {
                                                                    g.wallets.insert(new_name.clone(), v);
                                                                }
                                                                save_wallets(&g.wallets, &g.pin).unwrap();
                                                            });

                                                            editing_wallet.set(None);
                                                        }
                                                    }
                                                },

                                                onblur: move |_| {
                                                    editing_wallet.set(None);
                                                }
                                            }
                                        } else {
                                            span {
                                                class: "font-bold truncate flex items-center gap-2 justify-start text-left",

                                                img {
                                                    class: "invert rounded-none cursor-pointer",
                                                    src: WRITE_ICON,
                                                    onclick: move |_| {
                                                        edit_value.set(wallet.clone());
                                                        editing_wallet.set(Some(wallet.clone()));
                                                    }
                                                }
                                                "{wallet}"
                                            }
                                        }

                                        div {
                                            class: "flex items-center gap-5 justify-end w-full justify-self-end",
                                            div {
                                                class: "",
                                                CopyBox { title: "Wallet public key", text: public.dump_base36() }
                                                if Some(wallet_copy.clone()) == showing_private() {
                                                    CopyBox {
                                                        title: "Wallet private key",
                                                        class: "border-[var(--accent)]!",
                                                        text: private.dump_base36()
                                                    }
                                                }
                                            }

                                            img {
                                                title: "Show wallet private key",
                                                class: "rounded-none! invert cursor-pointer",
                                                onclick: move |_| {
                                                    let wallet = wallet_copy.clone();

                                                    if Some(wallet.clone()) == showing_private() {
                                                        showing_private.set(None);
                                                        return;
                                                    }

                                                    spawn(async move {
                                                        if ask_for_auth().await {
                                                            showing_private.set(Some(wallet.clone()));
                                                        }
                                                    });
                                                },
                                                src: PRIVATE_ICON,
                                            }
                                            if Some(wallet_copy2.clone()) == showing_private() {
                                                img {
                                                    title: "Delete wallet permanently",
                                                    class: "rounded-none! invert cursor-pointer",
                                                    onclick: move |_| {
                                                        let wallet = wallet_copy2.clone();

                                                        spawn(async move {
                                                            if ask_for_auth().await {
                                                                global_state.write().wallets.remove(&wallet);

                                                                if global_state().selected_wallet == wallet {
                                                                    if let Some((w, _p)) = global_state().wallets.clone().iter_mut().next() {
                                                                        global_state.write().selected_wallet = w.clone();
                                                                    } else {
                                                                        global_state.write().selected_wallet = "".to_string();
                                                                    }
                                                                }
                                                                save_wallets(&global_state().wallets, &global_state().pin).unwrap();
                                                            }
                                                        });
                                                    },
                                                    src: TRASH_ICON,
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div {
                        class: "w-full",

                        if creating_wallet() {
                            div {
                                class: "flex flex-col w-full gap-2 my-10",
                                h3 { "New Wallet" }
                                label { "Wallet name" }
                                input {
                                    type: "text",
                                    placeholder: "Main Wallet",
                                    onchange: move |e| {
                                        new_wallet_name.set(e.value());
                                    },
                                    value: new_wallet_name()
                                }
                                label { "Wallet private key" }
                                input {
                                    type: "text",
                                    class: "font-mono",
                                    placeholder: "Base36 wallet private key",
                                    onchange: move |e| {
                                        new_wallet_private.set(e.value());
                                    },
                                    value: new_wallet_private()
                                }
                                button {
                                    class: "w-full text-center",
                                    onclick: move |_| {
                                        if global_state().wallets.get(&new_wallet_name()).is_some() || new_wallet_name() == "".to_string() {
                                            new_wallet_error.set("Please choose a different wallet name".to_string());
                                            return;
                                        }

                                        new_wallet_error.set("".to_string());
                                        show_backup_popup.set(true);
                                        backup_countdown.set(10);

                                        spawn(async move {
                                            for i in (0..10).rev() {
                                                sleep(Duration::from_secs(1)).await;
                                                backup_countdown.set(i);
                                            }
                                        });
                                    },
                                    "Create"
                                }
                                p {
                                    class: "text-red-400! font-bold",
                                    "{new_wallet_error}"
                                }
                            }
                        }

                        button {
                            class: "w-full text-center".to_string() + if creating_wallet() { " bg-transparent! border-gray-600! border!" } else { "" },
                            onclick: move |_| {
                                creating_wallet.set(!creating_wallet());
                                new_wallet_private.set(Private::new_random().dump_base36())
                            },
                            if creating_wallet() { "Cancel" } else { "New wallet" }
                        }
                    }
                }
            }
        }
    }
}
