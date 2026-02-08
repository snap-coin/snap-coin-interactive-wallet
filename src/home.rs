use std::time::Duration;

use anyhow::anyhow;
use chrono::{Local, TimeZone};
use dioxus::prelude::*;
use snap_coin::{
    blockchain_data_provider::{BlockchainDataProvider, BlockchainDataProviderError},
    build_transaction,
    core::transaction::{TransactionId, MAX_TRANSACTION_IO},
    crypto::keys::{Private, Public},
    economics::DEV_WALLET,
    to_nano, to_snap, UtilError,
};
use tokio::time::sleep;

use crate::{
    annotate::AnnotateTransaction, authorize::ask_for_auth, copy_box::CopyBox, GlobalContext, Route,
};

const REFRESH: Asset = asset!("../assets/refresh.svg");
const FILE_CHECK: Asset = asset!("../assets/file_check.svg");

pub fn format_timestamp_secs(ts: u64) -> String {
    let dt = Local.timestamp_opt(ts as i64, 0).unwrap();
    dt.format("%b %d, %Y · %H:%M:%S").to_string()
}

struct HistoryTX {
    receivers: Vec<Public>,
    senders: Vec<Public>,
    is_send: bool,
    amount_snap: f64,
    tx: TransactionId,
    when: String,
}

#[component]
pub fn Home() -> Element {
    let navigator = use_navigator();
    let ctx = consume_context::<Signal<GlobalContext>>();

    if ctx().api_client.is_none() {
        navigator.push(Route::Login);
    }

    let client = ctx().api_client.unwrap();

    let resolve_wallet = move || {
        if let Some(p) = ctx().wallets.get(&ctx().selected_wallet) {
            *p
        } else {
            navigator.push(Route::WalletManager);
            Private::new_from_buf(&[0u8; 32])
        }
    };

    let mut private = use_signal(|| resolve_wallet());

    use_effect(move || {
        private.set(resolve_wallet());
    });

    let public = use_memo(move || private().to_public());
    let address = use_memo(move || public().dump_base36());

    let mut error = use_signal(|| "".to_string());
    let mut balance_snap = use_signal(|| 0f64);
    let mut tx_history: Signal<Vec<HistoryTX>> = use_signal(|| vec![]);
    let mut need_refresh = use_signal(|| false);

    // ---------------- SEND SIGNALS ----------------
    let mut recipients: Signal<Vec<(String, String)>> =
        use_signal(|| vec![(String::new(), String::new())]);

    let mut is_sending = use_signal(|| false);
    let mut ignore_inputs = use_signal(|| Vec::new());

    let total_amount = use_memo(move || {
        recipients()
            .iter()
            .filter_map(|(_, amt)| amt.parse::<f64>().ok())
            .sum::<f64>()
    });
    // ------------------------------------------------

    let mut tx_status = use_signal(|| "".to_string());

    let client_clone = client.clone();

    use_coroutine(move |_: UnboundedReceiver<()>| {
        let client = client_clone.clone();
        async move {
            loop {
                let private = private();
                let public = private.to_public();

                if let Err(e) = async {
                    let balance = to_snap(client.get_balance(public).await?);
                    let tx_ids = client.get_transactions_of_address(public, Some(2)).await?;

                    let mut history = vec![];

                    for tx_id in tx_ids.iter().take(10) {
                        let tx = client.get_transaction(tx_id).await?.expect("TX NOT FOUND");

                        let mut my_out = 0;
                        let mut my_in = 0;

                        for input in &tx.inputs {
                            if input.output_owner == public {
                                let fund_tx = client
                                    .get_transaction(&input.transaction_id)
                                    .await?
                                    .expect("TX NOT FOUND");

                                my_out += fund_tx.outputs[input.output_index].amount;
                            }
                        }

                        for output in &tx.outputs {
                            if output.receiver == public {
                                my_in += output.amount;
                            }
                        }

                        history.push(HistoryTX {
                            senders: tx.inputs.iter().map(|i| i.output_owner).collect(),
                            receivers: tx.outputs.iter().map(|o| o.receiver).collect(),
                            is_send: my_in < my_out,
                            amount_snap: to_snap((my_out as i64 - my_in as i64).abs() as u64),
                            tx: *tx_id,
                            when: format_timestamp_secs(tx.timestamp),
                        });
                    }

                    balance_snap.set(balance);
                    tx_history.set(history);
                    error.set("".into());

                    Ok::<(), BlockchainDataProviderError>(())
                }
                .await
                {
                    error.set(e.to_string());
                }

                for _ in 0..10 {
                    sleep(Duration::from_secs(1)).await;
                    if need_refresh() {
                        need_refresh.set(false);
                        break;
                    }
                }
            }
        }
    });
    let client = client.clone();

    let mut annotating_tx = use_signal(|| None);

    if let Some(tx) = annotating_tx() {
        return rsx! {
            div {
                button {
                    class: "p-20",
                    onclick: move |_| {
                        annotating_tx.set(None);
                    },
                    "Return"
                },
                AnnotateTransaction { transaction: tx }
            }
        };
    }

    rsx! {
        div {
            class: "w-full h-full p-6 text-white flex flex-col gap-6",

            div {
                class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold text-nowrap", "Home" }
                span {
                    class: "flex items-center gap-5",
                    CopyBox { text: address(), title: "Wallet Public Address (Receive)" }
                    img {
                        src: REFRESH,
                        class: "rounded-none! cursor-pointer invert".to_string() + if need_refresh() { " spin-fast" } else { "" },
                        onclick: move |_| need_refresh.set(true)
                    }
                }
            }

            div {
                class: "grid grid-cols-3 gap-6 flex-1",

                div {
                    class: "col-span-1 flex flex-col gap-6",

                    div {
                        class: "bg-neutral-900 rounded-xl p-6 shadow",
                        p { class: "text-sm text-neutral-400", "Balance" }
                        h2 { class: "text-3xl font-bold mt-2 font-mono", "{balance_snap} SNAP" }
                    }

                    // ---------------- SEND PANEL ----------------
                    div {
                        class: "bg-neutral-900 rounded-xl p-6 shadow flex flex-col gap-4",
                        h3 { class: "font-semibold text-lg", "Send" }

                        for (idx, (addr, amt)) in recipients.read().iter().enumerate() {
                            div { class: "flex gap-2",
                                div {
                                    class: "flex flex-col gap-3 w-full",
                                    input {
                                        class: "bg-neutral-800 p-2 rounded flex-1 w-full",
                                        placeholder: "Recipient address",
                                        value: "{addr}",
                                        oninput: move |e| {
                                            recipients.with_mut(|r| r[idx].0 = e.value());
                                        }
                                    }

                                    input {
                                        class: "bg-neutral-800 p-2 rounded w-full font-mono font-bold",
                                        type: "number",
                                        placeholder: "Amount",
                                        value: "{amt}",
                                        oninput: move |e| {
                                            recipients.with_mut(|r| r[idx].1 = e.value());
                                        }
                                    }
                                }

                                if recipients.read().len() > 1 {
                                    button {
                                        class: "px-2 text-red-400 bg-neutral-900! border! border-[var(--border)]!",
                                        onclick: move |_| {
                                            recipients.with_mut(|r| { r.remove(idx); });
                                        },
                                        "✕"
                                    }
                                }
                            }
                        }

                        button {
                            class: "text-sm text-indigo-400 self-start",
                            onclick: move |_| {
                                recipients.with_mut(|r| r.push((String::new(), String::new())));
                            },
                            "+ Add recipient"
                        }

                        div {
                            class: "text-sm text-neutral-400",
                            "Total: {total_amount:.4} SNAP"
                        }

                        button {
                            class: "bg-indigo-600 hover:bg-indigo-500 transition p-2 rounded font-semibold disabled:opacity-50",
                            disabled: is_sending(),
                            onclick: move |_| {
                                let client = client.clone();
                                spawn(async move {

                                    tx_status.set("".to_string());
                                    is_sending.set(true);

                                    let client_clone = client.clone();
                                    if let Err(e) = async move {
                                        if !ask_for_auth().await {
                                            return Err(anyhow!("Unauthorized"));
                                        }

                                        let mut receivers = vec![];
                                        for (r, a) in recipients() {
                                            receivers.push((Public::new_from_base36(&r).ok_or(anyhow!("Invalid receiver address"))?, to_nano(a.parse().map_err(|_| anyhow!("Invalid amount"))?)));
                                        }
                                        let mut ignore_inputs = ignore_inputs.write();
                                        tx_status.set("Building transaction...".to_string());
                                        let mut tx = build_transaction(&*client_clone, private(), receivers, &*ignore_inputs).await?;
                                        let used_inputs = tx.inputs.clone();
                                        tx_status.set("Computing transaction PoW...".to_string());
                                        tx.compute_pow(&client_clone.get_live_transaction_difficulty().await?, Some(0.2f64))?;
                                        tx_status.set("Submitting transaction...".to_string());
                                        client_clone.submit_transaction(tx).await??;
                                        ignore_inputs.extend(used_inputs);

                                        Ok::<(), anyhow::Error>(())
                                    }.await {
                                        if let Some(UtilError::TooMuchIO) = e.downcast_ref::<UtilError>() {
                                            if let Err(e) = async move {
                                                let available = client
                                                    .get_available_transaction_outputs(private().to_public())
                                                    .await?;
                                                let mut part_count = 0;
                                                for part in available.chunks(MAX_TRANSACTION_IO - 1) {
                                                    let amount = part.iter().fold(0, |acc, part| part.1.amount + acc);
                                                    let mut tx = build_transaction(
                                                        &*client,
                                                        private(),
                                                        vec![(private().to_public(), amount)],
                                                        &ignore_inputs.write(),
                                                    )
                                                    .await?;
                                                    tx_status.set("Computing Proof Of Work for transaction".to_string());
                                                    tx.compute_pow(&client.get_transaction_difficulty().await?, Some(0.1))?;
                                                    tx_status.set(
                                                        format!("Built transaction: {}",
                                                        tx.transaction_id.unwrap().dump_base36())
                                                    );

                                                    tx_status.set("Submitting transaction...".to_string());

                                                    let used_inputs = tx.inputs.clone();
                                                    client.submit_transaction(tx).await??;
                                                    ignore_inputs.write().extend_from_slice(&used_inputs);

                                                    part_count += 1;
                                                }

                                                tx_status.set(format!("Merged into: {} UTXOs, please re-submit transaction, after merge transactions are confirmed", part_count));

                                                Ok::<(), anyhow::Error>(())
                                            }.await {
                                                tx_status.set(format!("Failed to merge UTXOs: {}", e));
                                            }
                                        } else {
                                            tx_status.set(format!("{}", e));
                                        }
                                    } else {
                                        recipients.set(vec![(String::new(), String::new())]);
                                        tx_status.set("Transaction submitted".to_string());
                                    }

                                    is_sending.set(false);
                                });
                            },
                            if is_sending() { "Sending..." } else { "Send Transaction" }
                        }

                        p {
                            "{tx_status}"
                        }
                    }
                    // ------------------------------------------------
                }

                div {
                    class: "col-span-2 bg-neutral-900 rounded-xl p-6 shadow flex flex-col overflow-hidden",
                    h3 { class: "font-semibold text-lg mb-4", "Transaction History" }

                    div {
                        class: "flex flex-col gap-2 overflow-auto pr-2",

                        for tx in tx_history.read().iter() {
                            {
                                let mut sender_main = tx.senders.first().map(|s| s.dump_base36()).unwrap_or("network".into());
                                let mut receiver_main = tx.receivers.first().map(|r| r.dump_base36()).unwrap_or("network".into());
                                if sender_main == "0" { sender_main = "burn".to_string(); }
                                if receiver_main == "0" { receiver_main = "burn".to_string(); }
                                if sender_main == DEV_WALLET.dump_base36() { sender_main = "developer".to_string(); }
                                if receiver_main == DEV_WALLET.dump_base36() { receiver_main = "developer".to_string(); }

                                let sender_more = tx.senders.len().saturating_sub(1);
                                let receiver_more = tx.receivers.len().saturating_sub(1);

                                let amount_class = if tx.is_send { "text-red-400" } else { "text-green-400" };
                                let sign = if tx.is_send { "-" } else { "+" };
                                let amount_text = format!("{sign}{:.4}", tx.amount_snap);

                                let tx_id = tx.tx.clone();
                                let tx_id_clone = tx.tx.clone();
                                let sender_main_clone = sender_main.clone();
                                let receiver_main_clone = receiver_main.clone();

                                rsx! {
                                    div {
                                        class: "bg-neutral-800 p-4 rounded grid grid-cols-2 gap-x-4 gap-y-3 text-sm",

                                        span { class: "font-semibold text-xl font-bold {amount_class}", "{amount_text} SNAP" }
                                        div {
                                            class: "flex items-center gap-5",
                                            span { class: "text-neutral-500 text-xs whitespace-nowrap text-right", "{tx.when.clone()}" }
                                            div {
                                                class: "flex items-center gap-2 min-w-0",
                                                p {
                                                    class: "text-nowrap",
                                                    "Transaction ID"
                                                }
                                                CopyBox { onclick: move |_| {
                                                    let _ = webbrowser::open(&("https://explorer.snap-coin.net/tx/".to_string() + &tx_id_clone.dump_base36()));
                                                }, class: "w-full min-w-0", text: tx_id_clone.dump_base36(), title: "Transaction ID" }
                                                img {
                                                    src: FILE_CHECK,
                                                    class: "rounded-none! cursor-pointer invert".to_string() + if need_refresh() { " spin-fast" } else { "" },
                                                    onclick: move |_| annotating_tx.set(Some(tx_id))
                                                }
                                            }
                                        }

                                        span { class: "text-neutral-500 text-xs self-center", "From" }
                                        div {
                                            class: "flex items-center gap-2 min-w-0",
                                            CopyBox { onclick: move |_| {
                                                let _ = webbrowser::open(&("https://explorer.snap-coin.net/wallet/".to_string() + &sender_main));
                                            }, class: "w-full min-w-0", text: sender_main_clone, title: "Sender" }
                                            if sender_more > 0 { span { class: "text-neutral-500 shrink-0", "+{sender_more}" } }
                                        }

                                        span { class: "text-neutral-500 text-xs self-center", "To" }
                                        div {
                                            class: "flex items-center gap-2 min-w-0",
                                            CopyBox { onclick: move |_| {
                                                let _ = webbrowser::open(&("https://explorer.snap-coin.net/wallet/".to_string() + &receiver_main));
                                            }, class: "w-full min-w-0", text: receiver_main_clone, title: "Receiver" }
                                            if receiver_more > 0 { span { class: "text-neutral-500 shrink-0", "+{receiver_more}" } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            p { class: "p-10 text-red-400! font-bold", "{error}" }
        }
    }
}
