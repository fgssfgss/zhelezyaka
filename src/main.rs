use log4rs;
use log::{info, warn};

mod sqlite;
mod cmd;
mod telegram;
use cmd::CommandType;

#[tokio::main]
async fn main() {
    log4rs::init_file("config/log.yaml", Default::default()).unwrap();

    info!("Zhelezyaka 2.0");

    let mut sqlite = sqlite::SqliteDB::new("./database.db");
    let mut telegram = telegram::Telegram::new("");

    loop {
        telegram.serve(| input| {
            let cmdtype = cmd::CommandParser::parse_command(&input);

            info!("input txt {}", &input);

            // TODO: make it as blocking operation in tokio
            let answer = match cmdtype {
                CommandType::EGenerateByWord(s) => sqlite.select(&s),
                CommandType::EGetCountByWord(s) => {
                    if let Some(n) = sqlite.is_exist(&s) {
                        format!("Count {}", n)
                    } else {
                        format!("Empty word provided")
                    }
                }
                CommandType::EDisableForChat => { warn!("Not implemented!"); String::new() },
                CommandType::EEnableForChat => { warn!("Not implemented!"); String::new() },
                CommandType::ENoCommand => { sqlite.insert(&input); sqlite.select("") },
            };

            info!("ANSWER IS {}", &answer);

            answer
        }).await;
    }
}