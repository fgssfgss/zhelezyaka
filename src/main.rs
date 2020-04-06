use std::io;

mod sqlite;

fn main() {
    println!("Zhelezyaka 2.0");

    let mut sqlitedb = sqlite::SqliteDB::new("./database.db");

    loop {
        let mut input = String::new();

        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                println!("success, bytes read {}: {}", n, input);
                sqlitedb.select(&input);
                //sqlitedb.insert(&input);
                if let Some(n) = sqlitedb.is_exist("хуй") {
                    println!("count {}", n);
                } else {
                    println!("empty word provided");
                }
            }
            Err(error) => println!("error: {}", error),
        }
    }
}