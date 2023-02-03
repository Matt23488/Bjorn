use std::sync::Arc;

use serenity::async_trait;

use ws_protocol::{ClientApi, ClientApiHandler};

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

pub struct Canceller(pub Vec<ws_protocol::Canceller>);

impl Canceller {
    pub fn cancel(self) {
        self.0.into_iter().for_each(|c| c.cancel());
    }

    pub fn add(&mut self, canceller: ws_protocol::Canceller) {
        self.0.push(canceller);
    }
}

pub fn load_config<T>(base_path: &str) -> Option<T>
where
    T: DiscordGame + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    serde_json::from_str(
        std::fs::read_to_string(format!("{}/{}/config.json", base_path, T::id()))
            .ok()?
            .as_str(),
    )
    .ok()
}

pub struct DiscordGameSetupData {
    pub config_path: String,
    pub data: Arc<tokio::sync::RwLock<serenity::prelude::TypeMap>>,
    pub cache_and_http: Arc<serenity::CacheAndHttp>,
    pub addr: String,
}

#[async_trait]
pub trait DiscordGame {
    fn id() -> &'static str;
    fn command_group() -> &'static serenity::framework::standard::CommandGroup;
    fn setup(
        setup_data: DiscordGameSetupData,
        serenity_data: &mut serenity::prelude::TypeMap,
        canceller: &mut Canceller,
    ) -> Result<(), ()>;
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
        _ctx: &serenity::prelude::Context,
        _msg: &serenity::model::prelude::Message,
    ) {
    }
    async fn server_message(
        data: Arc<tokio::sync::RwLock<serenity::prelude::TypeMap>>,
        cache_and_http: Arc<serenity::CacheAndHttp>,
        message: <<Self::Handler as ClientApiHandler>::Api as ClientApi>::Message,
    );
}

#[macro_export]
macro_rules! use_data {
    ($data:expr, |$item:ident: $key:ty| $body:expr) => {{
        let $item = loop {
            let data = $data.read().await;
            let opt = data.get::<$key>().unwrap().lock().unwrap().take();

            if let Some($item) = opt {
                break $item;
            }

            drop(data);
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        };

        let result = $body;

        let data = $data.read().await;
        data.get::<$key>().unwrap().lock().unwrap().replace($item);
        drop(data);

        result
    }};
}
