use std::sync::{Arc, Mutex};

use serenity::async_trait;

use crate::{ClientApi, ClientApiHandler, WsClient, WsClientComponents, WsClientHandlerComponents};

pub enum Role {
    User,
    Admin,
}

impl Role {
    pub fn is_admin(&self) -> bool {
        match self {
            Role::Admin => true,
            _ => false,
        }
    }

    pub fn is_user(&self) -> bool {
        match self {
            Role::User => true,
            _ => false,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RoleConfig {
    admin: Vec<u64>,
    user: Vec<u64>,
}

impl RoleConfig {
    async fn has_role_static(
        valid_roles: &Vec<u64>,
        ctx: &serenity::prelude::Context,
        msg: &serenity::model::prelude::Message,
        guild_id: serenity::model::prelude::GuildId,
    ) -> bool {
        for role in valid_roles {
            match msg.author.has_role(ctx, guild_id, *role).await {
                Ok(has_role) if has_role => {
                    return true;
                }
                _ => (),
            }
        }

        false
    }

    pub async fn has_role(
        &self,
        ctx: &serenity::prelude::Context,
        msg: &serenity::model::prelude::Message,
        guild_id: serenity::model::prelude::GuildId,
        role: Role,
    ) -> bool {
        if role.is_admin() && RoleConfig::has_role_static(&self.admin, ctx, msg, guild_id).await {
            true
        } else if role.is_user()
            && RoleConfig::has_role_static(&self.user, ctx, msg, guild_id).await
        {
            true
        } else {
            false
        }
    }
}

// TODO: Look at moving this to its own workspace item.
#[async_trait]
pub trait GameConfig: serenity::prelude::TypeMapKey {
    type Config: serde::Serialize + for<'de> serde::Deserialize<'de>;
    type MessageHandler: BjornMessageHandler + Send + Sync;

    type Api: ClientApi;
    type ApiHandler: ClientApiHandler;

    fn id() -> &'static str;
    fn command_group() -> &'static serenity::framework::standard::CommandGroup;
    fn new_ws_clients(
        client: &serenity::Client,
    ) -> (
        WsClientComponents<Self::Api>,
        WsClientHandlerComponents<<Self::ApiHandler as ClientApiHandler>::Api, Self::ApiHandler>,
    );

    fn new(game_config: Self::Config) -> Self::Value;
    async fn has_necessary_permissions(
        &self,
        ctx: &serenity::prelude::Context,
        msg: &serenity::model::prelude::Message,
        role: Role,
    ) -> bool;
}

#[async_trait]
pub trait BjornMessageHandler {
    type Handler: ClientApiHandler;

    async fn client_message(
        ctx: &serenity::prelude::Context,
        msg: &serenity::model::prelude::Message,
    );
    async fn server_message(
        data: Arc<tokio::sync::RwLock<serenity::prelude::TypeMap>>,
        cache_and_http: Arc<serenity::CacheAndHttp>,
        message: <<Self::Handler as ClientApiHandler>::Api as ClientApi>::Message,
    );
}

#[macro_export]
macro_rules! use_data {
    ($data:expr, |$item:ident: $key:ty| $body:block) => {
        let $item = loop {
            let data = $data.read().await;
            let opt = data.get::<$key>().unwrap().lock().unwrap().take();

            if let Some($item) = opt {
                break $item;
            }

            drop(data);
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        };

        $body

        let data = $data.read().await;
        data.get::<$key>().unwrap().lock().unwrap().replace($item);
        drop(data);
    }
}

impl<Api> serenity::prelude::TypeMapKey for WsClient<Api>
where
    Api: ClientApi + 'static,
{
    type Value = Mutex<WsClient<Api>>;
}
