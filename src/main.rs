use std::io;

mod sqlite;
mod cmd;
use cmd::CommandType;

fn main() {
    println!("Zhelezyaka 2.0");

    let mut sqlitedb = sqlite::SqliteDB::new("./database.db");

    loop {
        let mut input = String::new();

        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                println!("Success, bytes read {}: {}", n, input);

                let cmdtype = cmd::CommandParser::parse_command(&input);

                match cmdtype {
                    CommandType::EGenerateByWord(s) => sqlitedb.select(&s),
                    CommandType::EGetCountByWord(s) => {
                        if let Some(n) = sqlitedb.is_exist(&s) {
                            println!("Count {}", n);
                        } else {
                            println!("Empty word provided");
                        }
                    }
                    CommandType::EDisableForChat => println!("Not implemented!"),
                    CommandType::EEnableForChat => println!("Not implemented!"),
                    CommandType::ENoCommand => sqlitedb.insert(&input),
                }
            }
            Err(error) => println!("Error: {}", error),
        }
    }
}