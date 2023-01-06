use stylist::yew::styled_component;

use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_hooks::prelude::*;

use wasm_bindgen_futures::JsFuture;
use web_sys::{console, window, Request, RequestInit, Response};

#[styled_component(App)]
pub fn app() -> Html {
    let counter = use_state(|| 0);

    let message = use_async(fetch_message());

    let onclick = {
        let counter = counter.clone();
        move |_| {
            let value = *counter + 1;
            counter.set(value);
        }
    };

    let get_message = {
        let message = message.clone();
        Callback::from(move |_| {
            message.run();
        })
    };

    let container_div = css!(
        r#"
        display: grid;
        place-items: center;
        width: 100vw;
        height: 100vh;
    "#
    );

    let counter_div = css!(
        r#"
        text-align: center;
    "#
    );

    let message_div = counter_div.clone();

    html! {
        <div class={container_div}>
            <div class={message_div}>
                <button onclick={get_message} disabled={message.loading}>{"Get Message"}</button>
                <h3>{"Message from web server:"}</h3>
                <span>
                {
                    if message.loading {
                        html! { "<Loading...>" }
                    } else {
                        html! {}
                    }
                }
                {
                    if let Some(data) = &message.data {
                        html! { data }
                    } else {
                        html! {}
                    }
                }
                {
                    if let Some(error) = &message.error {
                        html! { error }
                    } else {
                        html! {}
                    }
                }
                </span>
            </div>
            <div class={counter_div}>
                <button {onclick}>{"+1"}</button>
                <p>{*counter}</p>
            </div>
        </div>
    }
}

#[wasm_bindgen]
pub async fn fetch_message() -> Result<String, String> {
    let window = window().unwrap();

    let mut opts = RequestInit::new();
    opts.method("GET");

    let request = Request::new_with_str_and_init("http://127.0.0.1:64209/", &opts).unwrap();

    console::log_1(&"Fetching from server".into());
    match JsFuture::from(window.fetch_with_request(&request)).await {
        Ok(response) => {
            let response: Response = match response.try_into() {
                Ok(response) => response,
                _ => return Err(String::from("Invalid response")),
            };

            let text = JsFuture::from(response.text().unwrap()).await.unwrap();
            let text = text.as_string().unwrap();

            Ok(text)
        }
        Err(_) => Err(String::from("404")),
    }
}
