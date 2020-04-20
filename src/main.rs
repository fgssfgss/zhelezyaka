use log4rs;
use log::{info, trace};
use std::env;
mod sqlite;
mod cmd;
mod telegram;
mod user_management;
mod user;
use cmd::CommandType;
use telegram::TelegramActions::*;
use sqlite::SqliteDB;
use user_management::UserManager;
use lazy_static::*;

lazy_static! {
    static ref SQLITE_POOL: SqliteDB = {
        let dbpath = env::var("DATABASE_PATH").expect("DATABASE_PATH is not provided");
        sqlite::SqliteDB::new(&dbpath)
    };

    static ref USER_MANAGER: UserManager = {
        user_management::UserManager::new(&SQLITE_POOL.get_conn())
    };
}

#[tokio::main(threaded_scheduler, core_threads = 4, max_threads = 8)]
async fn main() {
    let logconfig = env::var("LOG_CONFIG").expect("LOG_CONFIG is not provided");
    let token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN is not provided");

    log4rs::init_file(&logconfig, Default::default()).unwrap();

    info!("Zhelezyaka 2.0");

    let telegram = telegram::Telegram::new(&token);

    loop {
        telegram.serve(|chat_id, input| {
            let sqlite = SQLITE_POOL.get_conn();
            let mut user_account = USER_MANAGER.get_user(&sqlite, &chat_id.to_string());
            let cmdtype = cmd::CommandParser::parse_command(&input);

            info!("user acc is {:?}", user_account);
            info!("ChatId <{}>: input txt {:?}", chat_id, &input);

            match cmdtype {
                CommandType::EGenerateByWord(s) => ReplyToMessage(sqlite.select(s)),
                CommandType::EGetCountByWord(s) => {
                    if let Some(n) = sqlite.is_exist(s) {
                        ReplyToMessage(format!("Count {}", n))
                    } else {
                        ReplyToMessage(format!("Empty word provided"))
                    }
                }
                CommandType::EDisableForChat => {
                    info!("disable bot for this chat {}", &user_account.user_id);
                    user_account.answer_mode = false;
                    USER_MANAGER.update_user(&sqlite, &user_account);
                    NoReply
                },
                CommandType::EEnableForChat => {
                    info!("enable bot for this chat {}", &user_account.user_id);
                    user_account.answer_mode = true;
                    USER_MANAGER.update_user(&sqlite, &user_account);
                    NoReply
                },
                CommandType::ENoCommand => {
                    sqlite.insert(input);
                    if user_account.answer_mode {
                        ReplyToChat(sqlite.select(String::new()))
                    } else {
                        NoReply
                    }
                },
                _ => { NoReply }
            }
        },
        |chat_id, input_text| {
            let sqlite = SQLITE_POOL.get_conn();
            info!("input txt {}", &input_text);
            let user_account = USER_MANAGER.get_user(&sqlite, &chat_id.to_string());

            info!("inserting... {:?}", user_account);
            if user_account.is_admin {
                trace!("inserting the text into db");
                sqlite.insert(input_text);
            }
        }).await;
    }
}
