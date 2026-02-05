use dioxus::prelude::*;
use rfd::FileDialog;
use snap_coin::{core::transaction::TransactionId, crypto::Signature, to_snap};
use std::fs;

use crate::{copy_box::CopyBox, GlobalContext, Route};

#[component]
pub fn AnnotateTransaction(transaction: TransactionId) -> Element {
    let mut global = consume_context::<Signal<GlobalContext>>();
    if global().api_client.is_none() {
        navigator().replace(Route::Connection);
        return rsx! {};
    }
    let client = global().api_client.unwrap();

    let mut tx = use_signal(|| None);
    let mut input_amounts = use_signal(|| None);
    let mut status = use_signal(|| "".to_string());

    use_effect(move || {
        let client = client.clone();
        spawn(async move {
            if let Err(e) = async move {
                if let Some(tx_d) = client.get_transaction_and_info(&transaction).await? {
                    tx.set(Some(tx_d.clone()));
                    let mut amounts = vec![];
                    for input in tx_d.transaction.inputs {
                        let funder_tx = client
                            .get_transaction(&input.transaction_id)
                            .await?
                            .expect("Could not find funder tx");
                        amounts.push(funder_tx.outputs[input.output_index].amount)
                    }
                    input_amounts.set(Some(amounts));
                } else {
                    status.set("Could not find transaction".to_string());
                }

                Ok::<(), anyhow::Error>(())
            }
            .await
            {
                status.set(e.to_string());
            }
        });
    });

    if tx().is_none() {
        return rsx! {};
    }
    let tx = tx.unwrap();

    let mut title = use_signal(|| "".to_string());
    let mut description = use_signal(|| "".to_string());

    let mut inputs: Signal<Vec<String>> = use_signal(|| {
        tx.transaction
            .inputs
            .iter()
            .map(|_| String::new())
            .collect()
    });
    let mut outputs: Signal<Vec<String>> = use_signal(|| {
        tx.transaction
            .outputs
            .iter()
            .map(|_| String::new())
            .collect()
    });

    if !status().is_empty() {
        rsx! {
            p {
                class: "p-10 text-red-400! font-bold",
                "{status}"
            }
        }
    } else {
        rsx! {
            div {
                class: "z-10 p-20 flex flex-col gap-5",
                h3 {
                    "Annotate Transaction"
                }
                CopyBox { title: "Transaction ID", text: transaction.dump_base36() }

                input {
                    value: "{title}",
                    onchange: move |e| {
                        title.set(e.value());
                    },
                    placeholder: "Transaction title..."
                }

                textarea {
                    value: "{description}",
                    onchange: move |e| {
                        description.set(e.value());
                    },
                    placeholder: "Transaction description..."
                }

                h4 { "Funders" }
                div {
                    class: "flex flex-col gap-5 w-full divide-x divide-x-gray-600",
                    for (i, input) in tx.transaction.inputs.iter().enumerate() {
                        {
                            rsx! {
                                div {
                                    class: "flex gap-5 items-center",
                                    CopyBox { text: input.output_owner.dump_base36() }
                                    input {
                                        class: format!("{}", if inputs()[i] == "=hidden=" { "hidden" } else { "" }),
                                        value: inputs()[i].clone(),
                                        onchange: move |e| {
                                            inputs.write()[i] = e.value();
                                        },
                                        placeholder: "Funder note..."
                                    }
                                    button {
                                        onclick: move |_| {
                                            if inputs()[i] == "=hidden=" {
                                                inputs.write()[i] = "".to_string();
                                            } else {
                                                inputs.write()[i] = "=hidden=".to_string();
                                            }
                                        },
                                        if inputs()[i] == "=hidden=" { "Show" } else { "Hide" }
                                    }
                                }
                            }
                        }
                    }
                }

                h4 { "Payees" }
                div {
                    class: "flex flex-col gap-5 w-full divide-x divide-x-gray-600",
                    for (i, output) in tx.transaction.outputs.iter().enumerate() {
                        {
                            rsx! {
                                div {
                                    class: "flex gap-5 items-center w-full",
                                    CopyBox { text: output.receiver.dump_base36() }
                                    input {
                                        class: format!("{}", if outputs()[i] == "=hidden=" { "hidden" } else { "" }),
                                        value: outputs()[i].clone(),
                                        onchange: move |e| {
                                            outputs.write()[i] = e.value();
                                        },
                                        placeholder: "Payee note..."
                                    }
                                    button {
                                        onclick: move |_| {
                                            if outputs()[i] == "=hidden=" {
                                                outputs.write()[i] = "".to_string();
                                            } else {
                                                outputs.write()[i] = "=hidden=".to_string();
                                            }
                                        },
                                        if outputs()[i] == "=hidden=" { "Show" } else { "Hide" }
                                    }
                                }
                            }
                        }
                    }
                }

                h4 { "Annotate sign, and export" }
                button {
                    onclick: move |_| {
                        let header = format!("Transaction Confirmation\n\n{title}\n{description}\n\nNetwork Information\nTransaction ID: {}\nIncluded in block: #{}: {}\n\nFunders\n", transaction.dump_base36(), tx.at_height, tx.in_block.dump_base36());

                        let mut funders = String::new();

                        for (i, input) in tx.transaction.inputs.iter().enumerate() {
                            if inputs()[i] == "=hidden=" {
                                continue;
                            }
                            let funder = format!("Funder #{}: {}\nSender address: {}\nSignature: {}\nTotal: -{} SNAP\n\n", i + 1, inputs()[i], input.output_owner.dump_base36(), input.signature.unwrap().dump_base36(), to_snap(input_amounts().unwrap()[i]));
                            funders += &funder;
                        }

                        let payee = format!("Payees\n");

                        let mut payees = String::new();

                        for (i, output) in tx.transaction.outputs.iter().enumerate() {
                            if outputs()[i] == "=hidden=" {
                                continue;
                            }
                            let payee = format!("Payee #{}: {}\nPayee address: {}\nTotal: +{} SNAP\n\n", i + 1, outputs()[i], output.receiver.dump_base36(), to_snap(output.amount));
                            payees += &payee;
                        }

                        let all = header + &funders + &payee + &payees;
                        let selected_wallet = global().selected_wallet;
                        let mut private = global.write().wallets[&selected_wallet];

                        let annotate_sign = Signature::new_signature(&mut private, all.as_bytes());

                        let footer = format!("Annotation Information\nCreated by: {}\nCreator signature: {}", private.to_public().dump_base36(), annotate_sign.dump_base36());

                        let export = all + &footer;

                        if let Some(path) = FileDialog::new()
                            .set_title("Save Annotated Transaction")
                            .set_file_name(&format!("tx-{}.txt", &transaction.dump_base36()[0..8]))
                            .add_filter("Text", &["txt"])
                            .save_file()
                        {
                            if let Err(e) = fs::write(&path, export) {
                                status.set(format!("Failed to save file: {e}"));
                            }
                        }
                    },
                    "Download"
                }
            }
        }
    }
}
