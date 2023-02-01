use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

use crate::client;

pub fn parse_line(line: &str, players: &Arc<Mutex<Vec<String>>>) -> Option<client::Message> {
    PARSERS.iter().find_map(|parser| parser(line, players))
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
        ($regex:expr $(, $players:ident)? $(,)?)

        = $pattern:pat_param
        $(if $condition:expr)?

        => $message:expr
    } => {{
        Box::new(|line, _players| {
            regex!(REGEX, $regex);

            $(let $players = _players;)?

            if let Some($pattern) = captures!(REGEX, line) {
                if $(($condition) &&)? true {
                    Some($message)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }};
}

type Parser = dyn Fn(&str, &Arc<Mutex<Vec<String>>>) -> Option<client::Message> + Send + Sync;

/// The closures in this vector are evaluated in order, so the first
/// successful match will be returned. Keep this in mind when
/// determining where to put new parsers.
static PARSERS: Lazy<Vec<Box<Parser>>> = Lazy::new(|| {
    vec![
        parser! { // TODO: This regex is from Minecraft and needs to be adjusted here. I think Valheim requires special logic for detecting player joins/disconnects.
            (r"([a-zA-Z0-9_]+) joined the game$", players) = [player] => {
                players.lock().unwrap().push(String::from(*player));
                client::Message::PlayerJoined(
                    String::from(*player),
                )
            }
        },
        parser! { // TODO: This regex is from Minecraft and needs to be adjusted here. I think Valheim requires special logic for detecting player joins/disconnects.
            (r"([a-zA-Z0-9_]+) left the game$", players) = [player] => {
                let mut players = players.lock().unwrap();
                let player_index = players.iter().position(|p| p == player).unwrap();
                players.remove(player_index);

                client::Message::PlayerQuit(
                    String::from(*player),
                )
            }
        },
        // parser! {
        //     (r"\[Server thread/INFO\]: ([a-zA-Z0-9_]+) (.+)$", players)

        //     = [player, death_message]
        //     if !death_message.starts_with("lost connection") && players.lock().unwrap().iter().find(|p| p == player).is_some()

        //     => client::Message::PlayerDied(
        //         String::from(*player),
        //         String::from(*death_message),
        //     )
        // },
        parser! {
            (r#"Session "\w+" with join code (\d{6}) and IP \d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}:2456 is active with \d+ player\(s\)$"#, players) = [code] => {
                players.lock().unwrap().clear();
                client::Message::StartupComplete(String::from(*code))
            }
        },
    ]
});
