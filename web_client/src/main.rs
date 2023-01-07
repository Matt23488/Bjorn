use std::rc::Rc;

use stylist::yew::{styled_component, Global};
use yew::prelude::*;

mod app;
use app::App;

mod ws_client;
use ws_client::WsClient;

#[styled_component(Index)]
fn index() -> Html {
    let ws_client = use_memo(|_| WsClient::new(), ());

    html! {
        <>
            <Global css={css!(r#"
                html, body {
                    padding: 0;
                    margin: 0;
                    font-family: sans-serif;
                }
            "#)} />
            <ContextProvider<Rc<WsClient>> context={ws_client}>
                <App />
            </ContextProvider<Rc<WsClient>>>
        </>
    }
}

fn main() {
    yew::Renderer::<Index>::new().render();
}
