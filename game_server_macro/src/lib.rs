// use proc_macro::TokenStream;
// use quote::{format_ident, quote};
// use syn;

// #[proc_macro_attribute]
// pub fn bjorn_command(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let attr = syn::parse(attr).unwrap();
//     let item = syn::parse(item).unwrap();

//     impl_bjorn_command(&attr, &item)
// }

// fn impl_bjorn_command(attr: &syn::LitStr, item: &syn::ItemFn) -> TokenStream {
//     if item.sig.asyncness.is_some() {
//         panic!("Async functions aren't supported. I have to figure out how to get around the lingering borrow of `data`.");
//     }

//     let mut clone = item.clone();
//     let name = &item.sig.ident;
//     clone.sig.ident = format_ident!("__{name}");
//     let clone_name = &clone.sig.ident;
//     let vis = &item.vis;
//     let attrs = &item.attrs;

//     let call = if item.sig.inputs.len() == 1 {
//         quote! {
//             #clone_name(ws)
//         }
//     } else if item.sig.inputs.len() == 2 {
//         quote! {
//             let rest: String = msg.content.chars().skip_while(|c| !c.is_whitespace()).collect();
//             #clone_name(ws, rest.trim())
//         }
//     } else {
//         panic!("Incompatible arguments");
//     };

//     let gen = quote! {
//         #clone

//         #[serenity::framework::standard::macros::command]
//         #(#attrs)*
//         #vis async fn #name(ctx: &serenity::client::Context, msg: &serenity::model::channel::Message) -> serenity::framework::standard::CommandResult {
//             let channel_ok = match env::var("BJORN_MINECRAFT_DISCORD_COMMAND_CHANNEL") {
//                 Err(_) => false,
//                 Ok(channel) => match channel.parse::<u64>() {
//                     Ok(channel) => channel == msg.channel_id.0,
//                     Err(_) => false,
//                 }
//             };

//             let user_ok = match env::var("BJORN_MINECRAFT_DISCORD_ADMIN") {
//                 Err(_) => false,
//                 Ok(admin) => match admin.parse::<u64>() {
//                     Ok(admin) => admin == msg.author.id.0,
//                     Err(_) => false,
//                 }
//             };

//             if !channel_ok || !user_ok {
//                 return Ok(());
//             }

//             let data = ctx.data.read().await;

//             // * NOTE: Using this if let expression lets us ensure that `data` will not be used after any additional awaits.
//             // * Otherwise the `command` macro errors.
//             let ws_closed = if let Some(ws) = data.get::<ws_protocol::client::Dispatcher>().unwrap().lock().unwrap().as_ref() {
//                 #call.is_err()
//             } else {
//                 true
//             };

//             msg.reply(
//                 ctx,
//                 if ws_closed {
//                     "No connection to Game Manager."
//                 } else {
//                     #attr
//                 },
//             )
//             .await?;

//             Ok(())
//         }
//     };
//     gen.into()
// }
