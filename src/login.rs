use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
};

use anyhow::anyhow;
use dioxus::prelude::*;

use crate::{
    authorize::ask_for_auth, encryption::decrypt_wallets, save_wallets, wallet_path, GlobalContext,
    Route,
};

#[component]
pub fn Login() -> Element {
    let first_login = !fs::exists(wallet_path().unwrap()).unwrap();
    let ctx = consume_context::<Signal<GlobalContext>>();
    let mut pin = use_signal(|| String::new());
    let mut error = use_signal(|| String::new());

    // --- YOU implement this later ---
    let on_pin_try = {
        let ctx = ctx.clone();
        move |pin: String| {
            let mut ctx = ctx.clone();
            async move {
                (async move {
                    if first_login {
                        ctx.write().pin = pin.clone();
                        if !ask_for_auth().await {
                            error.set("PINs do not match".to_string());
                            return Err(anyhow!(""));
                        }
                    }

                    let path = wallet_path()?;
                    if !path.exists() {
                        save_wallets(&HashMap::new(), &pin).unwrap();
                        return Ok(());
                    }
                    let mut file = File::open(path)?;
                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf)?;
                    if let Some(wallets) = decrypt_wallets(&buf, &pin) {
                        ctx.write().wallets = wallets.clone();
                        if let Some(wallet) = wallets.keys().next() {
                            ctx.write().selected_wallet = wallet.clone();
                        }
                        return Ok(());
                    }
                    return Err(anyhow!(""));
                })
                .await
                .is_ok()
            }
        }
    };

    let try_submit = {
        let pin = pin.clone();
        let error = error.clone();
        let on_pin_try = on_pin_try.clone();
        let ctx = ctx.clone();

        move || {
            let current = pin();

            spawn({
                let mut pin = pin.clone();
                let mut error = error.clone();
                let mut ctx = ctx.clone();
                let on_pin_try = on_pin_try.clone();

                async move {
                    if on_pin_try(current.clone()).await {
                        ctx.write().pin = current;
                        navigator().replace(Route::Connection);
                        error.set(String::new());
                    } else {
                        error.set("Invalid PIN".into());
                        pin.set(String::new());
                    }
                }
            });
        }
    };

    rsx! {
        div {
            class: "
                fixed inset-0
                flex items-center justify-center
            ",

            div {
                class: "
                    w-full max-w-md
                    border border-var(--border)
                    rounded-2xl
                    bg-var(--bg)
                    p-10
                    shadow-2xl
                ",

                div { class: "mb-6 text-center",
                    h1 { class: "text-2xl font-semibold text-var(--text)", if first_login { "Welcome" } else { "Welcome Back" } }
                    p { class: "text-sm text-var(--muted) mt-1", if first_login { "Create a PIN to continue" } else { "Enter your PIN to continue" } }
                }

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
                        px-4 py-4
                        text-2xl
                        tracking-widest
                        text-center
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
                    p { class: "mt-4 text-sm text-red-500 text-center", "{error()}" }
                }

                div { class: "mt-6 text-center text-xs text-var(--muted)",
                    "Encrypted Access"
                }

                div { class: "mt-6 text-gray-400 text-center text-xs text-var(--muted)",
                    "Cross-compatible with the Snap Coin Wallet CLI"
                }
            }
        }
    }
}
