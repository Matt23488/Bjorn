use stylist::yew::styled_component;

use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::ws_client::{WsConnection, WsClientContext};

#[styled_component(App)]
pub fn app() -> Html {
    let counter = use_state(|| 0);
    let message_input = use_node_ref();

    let ws_context = use_context::<WsClientContext>().expect("context to be there");

    let onclick = {
        let counter = counter.clone();
        move |_| {
            let value = *counter + 1;
            counter.set(value);
        }
    };

    let toggle_connection = {
        let ws_context = ws_context.clone();
        move |_| {
            match &*ws_context.connection {
                WsConnection::Connected => {
                    ws_context.disconnect.emit(());
                }
                _ => {
                    ws_context.try_connect.emit(());
                }
            }
        }
    };

    let send_text = {
        let ws_context = ws_context.clone();
        let message_input = message_input.clone();
        move |_| {
            let message_input = message_input.cast::<HtmlInputElement>().unwrap();
            ws_context.send_text.emit(message_input.value());
        }
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

    let ws_div = counter_div.clone();

    html! {
        <div class={container_div}>
            <div class={ws_div}>
                <button onclick={toggle_connection}>
                {
                    match &*ws_context.connection {
                        WsConnection::Connected => "Disconnect",
                        WsConnection::Disconnected => "Connect",
                    }
                }
                </button>
                {
                    if let WsConnection::Connected = &*ws_context.connection {
                        html!{
                            <>
                                <input ref={message_input} type="text" />
                                <button onclick={send_text}>{"Send Text"}</button>
                            </>
                        }
                    } else { html! {} }
                }
            </div>
            <div class={counter_div}>
                <button {onclick}>{"+1"}</button>
                <p>{*counter}</p>
            </div>
        </div>
    }
}
