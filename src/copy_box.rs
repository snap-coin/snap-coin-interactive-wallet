use dioxus::prelude::*;
use dioxus_clipboard::hooks::use_clipboard;

const COPY_ICON: Asset = asset!("../assets/copy.svg");
const COPY_DONE_ICON: Asset = asset!("../assets/copy_done.svg");

#[component]
pub fn CopyBox(
    #[props(into)] text: String,
    #[props(optional)] class: String,
    #[props(optional)] title: String,
    #[props(optional)] onclick: EventHandler<MouseEvent>,
) -> Element {
    let mut copied = use_signal(|| false);
    let mut rotating = use_signal(|| false);

    rsx! {
        span {
            title,
            class: "font-mono font-light border border-gray-600 p-2 break-all flex items-center gap-2 justify-between rounded-md min-w-0 {class}",
            onclick,

            span {
                class: "truncate overflow-x-clip min-w-0 flex-1",
                {text.clone()}
            },

            img {
                class: format!(
                    "invert rounded-none! cursor-pointer transition {}",
                    if rotating() { "spin-fast" } else { "" }
                ),
                src: if copied() { COPY_DONE_ICON } else { COPY_ICON },

                onclick: move |_| {
                    copied.set(true);
                    rotating.set(true);

                    // reset animation + copied state
                    spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
                        rotating.set(false);

                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        copied.set(false);
                    });

                    use_clipboard().set(text.clone()).unwrap();
                }
            }
        }
    }
}
