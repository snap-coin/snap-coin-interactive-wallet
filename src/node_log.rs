use std::time::Duration;

use dioxus::prelude::*;
use tokio::{fs, time::sleep};

use crate::GlobalContext;

#[component]
pub fn NodeLog(class: Option<String>) -> Element {
    let mut logs = use_signal(|| "No node logs yet...".to_string());
    let global_context = consume_context::<Signal<GlobalContext>>();

    spawn(async move {
        loop {
            sleep(Duration::from_secs(2)).await;
            if let Some(internal_node) = global_context().internal_node {
                let log = match fs::read_to_string(internal_node.log_file.clone()).await {
                    Ok(log) => log,
                    Err(e) => {
                        logs.set(format!(
                            "Could not read log file {:?}, error: {}",
                            internal_node.log_file, e
                        ));
                        continue;
                    }
                };
                logs.set(log);
            }
        }
    });

    rsx! {
        pre {
            class: format!(
                "p-10! w-full h-200 overflow-y-scroll bg-[var(--panel-soft)] outline outline-gray-600 rounded-2xl font-mono font-light whitespace-pre-wrap {}",
                class.unwrap_or_default()
            ),
            "{logs}"
        }
    }
}
