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
        user_management::UserManager::new(&mut SQLITE_POOL.get_conn())
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
            let mut sqlite = SQLITE_POOL.get_conn();
            let mut user_account = USER_MANAGER.get_user(&sqlite, &chat_id.to_string());
            let cmdtype = cmd::CommandParser::parse_command(&input);
            let table_name = &user_account.lexeme_table;

            info!("User account is {:?}", user_account);
            info!("ChatId <{}>: input txt {:?}", chat_id, &input);

            match cmdtype {
                CommandType::EGenerateByWord(s) => ReplyToMessage(sqlite.select(&table_name, s)),
                CommandType::EGetCountByWord(s) => {
                    if let Some(n) = sqlite.is_exist(&table_name, s) {
                        ReplyToMessage(format!("Count {}", n))
                    } else {
                        ReplyToMessage(format!("Empty word provided"))
                    }
                }
                CommandType::EDisableForChat => {
                    info!("Disable bot for this chat {}", &user_account.user_id);
                    user_account.answer_mode = false;
                    USER_MANAGER.update_user(&sqlite, &user_account);
                    NoReply
                },
                CommandType::EEnableForChat => {
                    info!("Enable bot for this chat {}", &user_account.user_id);
                    user_account.answer_mode = true;
                    USER_MANAGER.update_user(&sqlite, &user_account);
                    NoReply
                },
                CommandType::ENoCommand => {
                    sqlite.insert(&table_name, input);
                    if user_account.answer_mode {
                        ReplyToChat(sqlite.select(&table_name, String::new()))
                    } else {
                        NoReply
                    }
                },
                CommandType::EChangeLexemeTable(table) => {
                    sqlite.create_lexeme_table(&table);
                    {
                        user_account.lexeme_table;
                    }
                    user_account.lexeme_table = String::from(&table);
                    USER_MANAGER.update_user(&sqlite, &user_account);
                    ReplyToMessage(format!("Created table {}", &table))
                },
                CommandType::EGetLexemeTable => {
                    ReplyToMessage(format!("Your current lexeme table is: {}", &user_account.lexeme_table))
                },
                CommandType::EHelpCommand => {
                    ReplyToMessage(format!("JelezyakaBot 2.0:\n/q - query funny story this awesome bot :))))\n/on - enable answer mode for this room/chat\n/off - disable answer mode for this room/chat\n/count - count word in your lexeme table\n/help - this help\n"))
                }, 
                CommandType::EAdminHelpCommand => {
                    ReplyToMessage(format!("EBALO AUF NUL!\n/adminhelp - only if you're admin of this bot\n/changetable - change lexeme table for this room/chat\n/getcurrenttable - get current table for this room/chat\n/listtable - list of lexeme tables\n"))
                },
                CommandType::EListLexemeTables => {
                    ReplyToMessage(format!("List of lexeme tables - {}", sqlite.fetch_lexems_tables_list().join(",")))
                },
                _ => { NoReply }
            }
        },
        |chat_id, input_text| {
            let sqlite = SQLITE_POOL.get_conn();
            info!("input txt {}", &input_text);
            let user_account = USER_MANAGER.get_user(&sqlite, &chat_id.to_string());
            let table_name = String::from("default");

            info!("inserting... {:?}", user_account);
            if user_account.is_admin {
                trace!("inserting the text into db");
                sqlite.insert(&table_name, input_text);
            }
        }).await;
    }
}
