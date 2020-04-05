use std::io;

mod sqlite;

fn main() {
    println!("Zhelezyaka 2.0");

    let mut sqlitedb = sqlite::SqliteDB::new();

    loop {
        let mut input = String::new();

        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                println!("success, bytes read {}: {}", n, input);
                sqlitedb.insert(&input);
                sqlitedb.select("huy");
                if let Some(n) = sqlitedb.is_exist("1") {
                    println!("count {}", n);
                } else {
                    println!("empty word provided");
                }
            }
            Err(error) => println!("error: {}", error),
        }
    }
}