use std::sync::Mutex;

use crate::{ClientApi, WsClient};

impl<Api> serenity::prelude::TypeMapKey for WsClient<Api>
where
    Api: ClientApi + 'static,
{
    type Value = Mutex<WsClient<Api>>;
}
