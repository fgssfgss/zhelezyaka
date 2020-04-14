use log4rs;
use log::{info, warn};
use std::env;
mod sqlite;
mod cmd;
mod telegram;
use cmd::CommandType;
use telegram::TelegramActions::*;

// TODO: how to add here more threads?
#[tokio::main]
async fn main() {
    let dbpath = env::var("DATABASE_PATH").expect("DATABASE_PATH is not provided");
    let logconfig = env::var("LOG_CONFIG").expect("LOG_CONFIG is not provided");
    let token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN is not provided");

    log4rs::init_file(&logconfig, Default::default()).unwrap();

    info!("Zhelezyaka 2.0");

    let sqlite = sqlite::SqliteDB::new(&dbpath);
    let mut telegram = telegram::Telegram::new(&token);

    loop {
        telegram.serve(|chat_id, input| {
            let cmdtype = cmd::CommandParser::parse_command(&input);

            info!("ChatId <{}>: input txt {}", chat_id, &input);

            match cmdtype {
                CommandType::EGenerateByWord(s) => ReplyToMessage(sqlite.select(s)),
                CommandType::EGetCountByWord(s) => {
                    if let Some(n) = sqlite.is_exist(s) {
                        ReplyToMessage(format!("Count {}", n))
                    } else {
                        ReplyToMessage(format!("Empty word provided"))
                    }
                }
                CommandType::EDisableForChat => { warn!("Not implemented!"); NoReply },
                CommandType::EEnableForChat => { warn!("Not implemented!"); NoReply },
                CommandType::ENoCommand => { sqlite.insert(input); ReplyToChat(sqlite.select(String::new())) },
            }
        }).await;
    }
}
