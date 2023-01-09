use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebSocket, console, MessageEvent};
use yew::prelude::*;

macro_rules! console_log {
    ($($t:tt)*) => (console::log_1(&format_args!($($t)*).to_string().as_str().into()))
}

#[derive(Clone, PartialEq)]
pub enum WsConnection {
    Disconnected,
    Connected,
}

impl WsConnection {
    fn connect(uri: &str, setter: UseStateSetter<WsConnection>) -> Result<WebSocket, &'static str> {
        let ws = match WebSocket::new(uri) {
            Ok(ws) => ws,
            _ => return Err("Failed to create WebSocket due to malformed `url`"),
        };

        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        let handshake_successful = Rc::new(RefCell::new(false));
        let ws_ = ws.clone();
        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(message) = e.data().dyn_into::<js_sys::JsString>() {
                if *handshake_successful.borrow() {
                    console_log!("Received {message}");
                } else if message.loose_eq(&"Bjorn".into()) {
                    console_log!("Hanshake with Bjorn WS successful");
                    *handshake_successful.borrow_mut() = true;
                    ws_.send_with_str("web").unwrap();
                } else {
                    console_log!("Connection doesn't appear to be bjorn (received {message}). Aborting.");
                    ws_.close_with_code_and_reason(1000, "Unknown server").unwrap();
                }
            } else {
                console::log_2(&"Unknown message received:".into(), &e.data());
            }
        });
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        let onerror_callback = {
            let setter = setter.clone();
            Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
                console_log!("ws error event: {:?}", e);
                setter.set(WsConnection::Disconnected);
            })
        };
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            console_log!("Socket opened");
            setter.set(WsConnection::Connected);
        });
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        Ok(ws)
    }
}

#[derive(Properties, PartialEq)]
pub struct WsClientProviderProps {
    pub children: Children,
}

#[function_component(WsClientProvider)]
pub fn ws_client_provider(props: &WsClientProviderProps) -> Html {
    let context = use_ws_client();

    html! {
        <ContextProvider<WsClientContext> {context}>
            { for props.children.iter() }
        </ContextProvider<WsClientContext>>
    }
}

#[derive(Clone, PartialEq)]
pub struct WsClientContext {
    pub connection: UseStateHandle<WsConnection>,
    pub try_connect: Callback<(), ()>,
    pub disconnect: Callback<(), ()>,
    pub send_text: Callback<String, ()>,
}

const SERVER_URL: &str = "ws://127.0.0.1:42069";

#[hook]
fn use_ws_client() -> WsClientContext {
    let connection = use_state(|| WsConnection::Disconnected);
    let ws = use_state(|| None);

    let try_connect: Callback<(), ()> = {
        let connection = connection.clone();
        let ws = ws.clone();
        Callback::from(move |_| {
            ws.set(Some(WsConnection::connect(SERVER_URL, connection.setter()).unwrap()));
        })
    };

    let disconnect: Callback<(), ()> = {
        let connection = connection.clone();
        let ws_opt = ws.clone();
        Callback::from(move |_| {
            if let (WsConnection::Connected, Some(ws)) = (&*connection, &*ws_opt) {
                ws.close_with_code_and_reason(1000, "Manual disconnect").unwrap();
                connection.set(WsConnection::Disconnected);
                ws_opt.set(None);
            }
        })
    };

    let send_text: Callback<String, ()> = {
        let connection = connection.clone();
        let ws = ws.clone();
        Callback::from(move |text: String| {
            if let (WsConnection::Connected, Some(ws)) = (&*connection, &*ws) {
                ws.send_with_str(text.as_str()).unwrap();
            }
        })
    };

    WsClientContext {
        connection,
        try_connect,
        disconnect,
        send_text
    }
}