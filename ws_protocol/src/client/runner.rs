use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::{message::Message, ApiSpecifier, Handshake, WsTask};

use super::mpsc::Endpoint;

pub struct Runner {
    api_specifier: ApiSpecifier,
    endpoint: Endpoint,
    on_cancel: oneshot::Receiver<()>,
}

impl Runner {
    pub fn new(
        api_specifier: ApiSpecifier,
        endpoint: Endpoint,
        on_cancel: oneshot::Receiver<()>,
    ) -> Runner {
        Runner {
            api_specifier,
            endpoint,
            on_cancel,
        }
    }
}

#[async_trait]
impl WsTask for Runner {
    async fn run(self, addr: String) {
        let Runner {
            api_specifier,
            endpoint,
            on_cancel,
        } = self;
        let (to_client, mut from_client) = endpoint.split();

        let cancel_task = tokio::spawn(on_cancel);

        // TODO: If there is no server connection, and a message is sent by the client, it
        // TODO: will be trapped in the mpsc channel until a connection is made, at which point
        // TODO: it will be send immediately to the server. I may need to find a way
        // TODO: to empty the channel before each connection attempt. Or otherwise
        // TODO: allow the client to ask if there is a connection first.
        let ws_task = async move {
            loop {
                if let Err(e) = connect(&addr, &api_specifier, &to_client, &mut from_client).await {
                    println!("WS connection failure: {e}");
                }

                println!("No connection. Trying again in 5 seconds...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        };

        tokio::select! {
            _ = cancel_task => {},
            _ = ws_task => {},
        };
    }
}

async fn connect(
    addr: &String,
    api_specifier: &ApiSpecifier,
    to_client: &UnboundedSender<Message>,
    from_client: &mut UnboundedReceiver<Message>,
) -> Result<(), Error> {
    let (ws_stream, _) = tokio_tungstenite::connect_async(addr).await?;
    println!("Established WS connection.");

    let (mut write, read) = ws_stream.split();

    let mut read = Box::pin(read);
    match read.next().await {
        Some(Ok(message)) => {
            if let Ok(Handshake::ServerIdentification) = message.try_into() {
                println!("Bjorn server identified.");
            } else {
                return Err(Error::InvalidHandshakeToken);
            }
        }
        _ => return Err(Error::InvalidHandshakeToken),
    }

    println!("Sending handshake response...");
    write
        .send(Handshake::ClientIdentification(api_specifier.clone()).into())
        .await?;

    println!("Bjorn handhsake complete.");

    let send_task = async move {
        loop {
            let message = match from_client.recv().await {
                Some(msg) => msg,
                None => break,
            };

            write.send(message.into()).await.unwrap_or_default();
        }
    };

    let to_client = to_client.clone();
    let recv_task = async move {
        read.try_for_each(|message| {
            let to_client = to_client.clone();
            async move {
                let message = match message.clone().try_into() {
                    Ok(msg) => msg,
                    Err(_) => {
                        println!("Received invalid WS message: {message}");
                        return Err(tungstenite::Error::ConnectionClosed);
                    }
                };

                to_client.send(message).unwrap_or_default();

                Ok(())
            }
        })
        .await
        .unwrap_or_default();
    };

    tokio::select! {
        _ = send_task => {}
        _ = tokio::spawn(recv_task) => {}
    }

    Ok(())
}

#[derive(Debug)]
enum Error {
    TungsteniteError(tungstenite::Error),
    InvalidHandshakeToken,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::TungsteniteError(e) => format!("Error in tungstenite: {e}"),
                Self::InvalidHandshakeToken => "Server sent invalid handshake token.".into(),
            }
        )
    }
}

impl std::error::Error for Error {}

impl From<tungstenite::Error> for Error {
    fn from(e: tungstenite::Error) -> Self {
        Error::TungsteniteError(e)
    }
}
