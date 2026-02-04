use std::sync::Arc;

use crate::GlobalContext;
use dioxus::prelude::*;
use futures_channel::oneshot;
use tokio::sync::Mutex;

pub async fn ask_for_auth() -> bool {
    let mut ctx = consume_context::<Signal<GlobalContext>>();

    let (tx, rx) = oneshot::channel();

    {
        let mut w = ctx.write();
        w.show_auth = true;
        w.auth_tx = Some(Arc::new(Mutex::new(Some(tx))));
    }

    rx.await.unwrap_or(false)
}

#[component]
pub fn Authorize() -> Element {
    let ctx = consume_context::<Signal<GlobalContext>>();
    let correct_pin = ctx().pin.clone();

    let mut pin = use_signal(|| String::new());
    let error = use_signal(|| "".to_string());

    let mut close = {
        let mut ctx = ctx.clone();
        let mut pin = pin.clone();
        let mut error = error.clone();
        move || {
            ctx.write().show_auth = false;
            pin.set(String::new());
            error.set(String::new());
        }
    };

    let mut try_submit = {
        let mut pin = pin.clone();
        let mut error = error.clone();
        let mut ctx = ctx.clone();
        let correct = correct_pin.clone();
        move || {
            if pin() == correct {
                if let Some(tx) = ctx.write().auth_tx.take() {
                    if let Ok(mut lock) = tx.try_lock() {
                        if let Some(tx) = lock.take() {
                            let _ = tx.send(true);
                        }
                    }
                }

                ctx.write().show_auth = false;
                pin.set(String::new());
                error.set(String::new());
            } else {
                error.set("Invalid PIN".into());
                pin.set(String::new());
            }
        }
    };

    if !ctx().show_auth {
        return rsx! {};
    }

    rsx! {
        div { class: "fixed inset-0 flex items-center justify-center bg-black/20",

            div { class: "relative max-w-500 border border-gray-400 rounded-xl bg-[var(--bg)] p-20 shadow-xl",

                button {
                    class: "absolute right-3 top-2 p-1! text-2xl text-[#ff7518]! bg-transparent! border-none text-var(--muted) hover:text-var(--text)",
                    onclick: move |_| close(),
                    "✕"
                }

                h2 { class: "text-lg font-semibold mb-1 text-var(--text)", "Authorization Required" }
                p { class: "text-sm text-var(--muted) mb-4", "Enter your 6-digit PIN" }

                input {
                    type: "password",
                    autofocus: true,
                    value: "{pin()}",
                    inputmode: "numeric",
                    maxlength: "6",
                    placeholder: "••••••",
                    class: "
                        w-full
                        bg-var(--panel)
                        border
                        border-var(--border)
                        text-var(--text)
                        placeholder-var(--muted)
                        rounded-lg
                        px-4 py-3
                        text-xl
                        focus:border-var(--accent)
                        focus:bg-var(--panel-soft)
                        focus:ring-2 focus:ring-var(--accent)/40
                        transition
                        duration-200
                    ",

                    oninput: move |evt| {
                        let v = evt.value();
                        if v.chars().all(|c| c.is_ascii_digit()) && v.len() <= 6 {
                            pin.set(v.clone());
                            if v.len() == 6 {
                                try_submit();
                            }
                        }
                    }
                }

                if !error().is_empty() {
                    p { class: "mt-3 text-sm text-red-500", "{error()}" }
                }
            }
        }
    }
}
