use stylist::yew::{styled_component, Global};
use yew::prelude::*;

mod app;
use app::App;

mod ws_client;
use ws_client::WsClientProvider;


#[styled_component(Index)]
fn index() -> Html {
    html! {
        <>
            <Global css={css!(r#"
                html, body {
                    padding: 0;
                    margin: 0;
                    font-family: sans-serif;
                }
            "#)} />
            <WsClientProvider>
                <App />
            </WsClientProvider>
        </>
    }
}

fn main() {
    yew::Renderer::<Index>::new().render();
}
