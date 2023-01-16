use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{self, parse_macro_input, spanned::Spanned};

#[proc_macro_attribute]
pub fn bjorn_command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as syn::AttributeArgs);
    let item = parse_macro_input!(item as syn::ItemFn);

    impl_bjorn_command(&attr, &item)
}

fn impl_bjorn_command(attr: &syn::AttributeArgs, item: &syn::ItemFn) -> TokenStream {
    let mut user_fn = item.clone();
    user_fn.sig.ident = format_ident!("bjorn_command_{}", item.sig.ident);

    let command_name = item.sig.ident.clone();
    let user_fn_ident = user_fn.sig.ident.clone();

    let mut admin_already_set = false;
    let mut role_path = quote!(ws_protocol::client::Role::User);
    let mut config = None;

    let mut admin_err = None;
    let mut config_err = None;
    for arg in attr {
        match arg {
            syn::NestedMeta::Meta(syn::Meta::Path(path)) => {
                let name = path.segments.first().as_ref().unwrap().ident.clone();
                if name.to_string() == "admin" {
                    if admin_already_set {
                        admin_err = Some(quote_spanned! {
                            path.span() => compile_error!("You've already declared admin on this command.");
                        });
                    } else {
                        role_path = quote!(ws_protocol::client::Role::Admin);
                        admin_already_set = true;
                    }

                    continue;
                };

                if config.is_some() {
                    config_err = Some(quote_spanned! {
                        path.span() => compile_error!("Config type already set.");
                    });

                    continue;
                }

                config = Some(quote!(#path));
            }
            _ => println!("Unknown arg, ignoring"),
        }
    }

    quote! {
        #admin_err
        #config_err

        #user_fn

        #[command]
        pub async fn #command_name(ctx: &serenity::prelude::Context, msg: &serenity::model::prelude::Message) -> CommandResult {
            let data = ctx.data.read().await;

            let config = data.get::<#config>().unwrap().lock().unwrap().take().unwrap();
            drop(data);

            if !config.has_necessary_permissions(ctx, msg, #role_path).await {
                msg.reply(ctx, "You don't have permission.").await.unwrap();

                let data = ctx.data.read().await;
                data.get::<#config>().unwrap().lock().unwrap().replace(config);
                drop(data);

                return Ok(());
            }

            let data = ctx.data.read().await;
            data.get::<#config>().unwrap().lock().unwrap().replace(config);
            drop(data);

            #user_fn_ident(ctx, msg).await
        }
    }.into()
}
