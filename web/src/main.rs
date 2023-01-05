use stylist::yew::{styled_component, Global};
use yew::prelude::*;

mod app;
use app::App;

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
            <App />
        </>
    }
}

fn main() {
    yew::Renderer::<Index>::new().render();
}
