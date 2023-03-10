use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

use crate::client;

use super::ConnectedPlayer;

pub fn parse_line(line: &str, state: ServerState) -> Option<client::Message> {
    PARSERS.iter().find_map(|parser| parser(line, &state))
}

pub struct ServerState {
    pub players: Arc<Mutex<Vec<ConnectedPlayer>>>,
    pub running: Arc<Mutex<bool>>,
    pub crossplay: bool,
}

macro_rules! captures {
    ($re:expr, $line:expr) => {
        $re.captures($line)
            .map(|captures| {
                captures
                    .iter()
                    .skip(1)
                    .flat_map(|c| c)
                    .map(|c| c.as_str())
                    .collect::<Vec<_>>()
            })
            .as_ref()
            .map(|c| c.as_slice())
    };

    (all, $re:expr, $line:expr) => {
        $re.captures($line)
            .map(|captures| {
                captures
                    .iter()
                    .flat_map(|c| c)
                    .map(|c| c.as_str())
                    .collect::<Vec<_>>()
            })
            .as_ref()
            .map(|c| c.as_slice())
    };
}

macro_rules! regex {
    ($name:ident, $regex:expr) => {
        static $name: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new($regex).unwrap());
    };
}

// Not sure if the pattern here is considered "good" or not
// but it works.
macro_rules! parser {
    {
        ($regex:expr $(, $state:pat_param)? $(,)?)

        = $pattern:pat_param
        $(if $condition:expr)?

        => $message:expr
    } => {{
        Box::new(|line, _state| {
            regex!(REGEX, $regex);

            $(let $state = _state;)?

            if let Some($pattern) = captures!(REGEX, line) {
                if $(($condition) &&)? true {
                    $message
                } else {
                    None
                }
            } else {
                None
            }
        })
    }};
}

type Parser = dyn Fn(&str, &ServerState) -> Option<client::Message> + Send + Sync;

/// The closures in this vector are evaluated in order, so the first
/// successful match will be returned. Keep this in mind when
/// determining where to put new parsers.
static PARSERS: Lazy<Vec<Box<Parser>>> = Lazy::new(|| {
    vec![
        parser! {
            (r": Got character ZDOID from ([\w\d_]+) : (.{2,}):\d+$", ServerState { players, .. }) = [player, id] => {
                let mut players = players.lock().unwrap();

                match players.iter().position(|p| p.id == *id) {
                    Some(_) => Some(client::Message::PlayerDied(String::from(*player))),
                    None => {
                        players.push(ConnectedPlayer {
                            id: String::from(*id),
                            name: String::from(*player),
                        });

                        Some(client::Message::PlayerJoined(
                            String::from(*player),
                        ))
                    }
                }

            }
        },
        parser! {
            (r": Destroying abandoned non persistent zdo \S+ owner (.+)$", ServerState { players, .. }) = [id] => {
                let mut players = players.lock().unwrap();
                let player_index = match players.iter().position(|p| p.id == *id) {
                    Some(idx) => idx,
                    None => return None,
                };
                let player = players.remove(player_index);

                Some(client::Message::PlayerQuit(
                    player.name,
                ))
            }
        },
        parser! {
            (r#"Session "\w+" with join code (\d{6}) and IP \d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}:2456 is active with \d+ player\(s\)$"#, ServerState { players, running, crossplay }) = [code] => {
                if *running.lock().unwrap() || !*crossplay {
                    return None;
                }

                players.lock().unwrap().clear();
                Some(client::Message::StartupComplete(Some(String::from(*code))))
            }
        },
        parser! {
            (r"^\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}: Game server connected$", ServerState { players, running, crossplay }) = [] => {
                if *running.lock().unwrap() || *crossplay {
                    return None;
                }

                players.lock().unwrap().clear();
                Some(client::Message::StartupComplete(None))
            }
        },
        parser! {
            (r"Random event set:(.+)$") = [event_id] => Some(client::Message::MobAttack(String::from(*event_id)))
        },
    ]
});
