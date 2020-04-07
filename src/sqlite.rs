use rusqlite::{Connection, params, CachedStatement, ToSql, Error};

const CREATE_DB: &str = "CREATE TABLE IF NOT EXISTS lexems (\
                            `lexeme1` TEXT, \
                            `lexeme2` TEXT, \
                            `lexeme3` TEXT, \
                            `count` INT NOT NULL DEFAULT '0', \
                            UNIQUE (`lexeme1`, `lexeme2`, `lexeme3`))";
const IS_EXIST: &str = "SELECT count FROM lexems \
                         WHERE lexeme1 = ?1 OR lexeme2 = ?1 OR lexeme3 = ?1;";
const SELECT_LEFT: &str = "SELECT lexeme1, lexeme2, lexeme3 FROM lexems \
                           WHERE lexeme2 = ?1 AND lexeme3 = ?2 ORDER BY RANDOM() DESC LIMIT 0,1;";
const SELECT_RIGHT: &str = "SELECT lexeme1, lexeme2, lexeme3 FROM lexems \
                            WHERE lexeme1 = ?1 AND lexeme2 = ?2 ORDER BY RANDOM() DESC LIMIT 0,1;";
const SELECT_LEXEME: &str = "SELECT lexeme1, lexeme2, lexeme3 FROM lexems \
                             WHERE lexeme1 = ?1 OR lexeme2 = ?1 OR lexeme3 = ?1 \
                             ORDER BY RANDOM() LIMIT 0,1;";
const SELECT_BEGIN: &str = "SELECT lexeme1, lexeme2, lexeme3 FROM lexems \
                            WHERE lexeme1 = '#beg#' ORDER BY RANDOM() LIMIT 0,1;";
// begin and end markers for text
const BEGIN: &str = "#beg#";
const END: &str = "#end#";

// maximum length of recursion limit in select_left and select_right functions
const MAXIMUM_RECURSION_DEPTH: i32 = 500;

pub struct SqliteDB {
    conn: Connection
}

fn query_statement<P>(stmt: &mut CachedStatement<'_>, params: P) -> Vec<String>
where
    P: IntoIterator,
    P::Item: ToSql,
{
    match stmt.query_row(params, |row| {
        let lexeme1: String = row.get(0).unwrap();
        let lexeme2: String = row.get(1).unwrap();
        let lexeme3: String = row.get(2).unwrap();
        Ok(vec![lexeme1, lexeme2, lexeme3])
    }) {
        Ok(v) => v,
        Err(_) => return vec![String::from(BEGIN),
                                     String::from("Not found"),
                                     String::from(END)],
    }
}

impl SqliteDB {
    pub fn new(path: &str) -> SqliteDB {
        println!("SqliteDB starting");
        let conn = Connection::open(path).unwrap();
        conn.execute(CREATE_DB, params![]).unwrap();
        SqliteDB { conn }
    }

    #[allow(dead_code)]
    pub fn is_exist(&mut self, word: &str) -> Option<i32> {
        if !word.is_empty() {
            let mut stmt = self.conn.prepare_cached(IS_EXIST).unwrap();
            let count = stmt.query_and_then(params![&word], |row| {
                let cnt: i32 = row.get(0).unwrap();
                Ok(cnt)
            }).unwrap().
                fold(0, |a, b: std::result::Result<i32, Error>| a + b.unwrap());

            Some(count)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn insert(&mut self, text: &String) -> () {
        let mut splitted: Vec<&str> = text.trim().split_whitespace().collect();

        splitted.insert(0, BEGIN);
        splitted.push(END);

        for s in &splitted {
            println!("{}", s);
        }

        for x in 0..(splitted.len() - 2) {
            let insert_sql = format!("INSERT OR IGNORE INTO lexems (\
                                       `lexeme1`, \
                                       `lexeme2`, \
                                       `lexeme3`) \
                                       VALUES ('{}', '{}', '{}');",
                                      splitted[x], splitted[x + 1], splitted[x + 2],
                                      );

            let update_sql = format!("UPDATE lexems \
                                             SET count = count+1 \
                                             WHERE \
                                             lexeme1 = '{}' AND lexeme2 = '{}' AND lexeme3 = '{}';",
                                     splitted[x], splitted[x + 1], splitted[x + 2]);

            println!("Inserting {}", insert_sql);
            match self.conn.execute(&insert_sql, params![]) {
                Err(e) => println!("Insert was unsuccessful: {}", e),
                _ => println!("Insert is successful"),
            }

            println!("Inserting {}", update_sql);
            match self.conn.execute(&update_sql, params![]) {
                Err(e) => println!("Insert was unsuccessful: {}", e),
                _ => println!("Insert is successful"),
            }
        }
    }

    pub fn select(&self, input: &str) -> () {
        let word= input.trim();

        if word.is_empty() {
            let result = self.select_random();
            println!("Found by random: {}", result);
        } else {
            let result = self.select_lexeme(word);
            println!("Found by word {}: {}", word, result);
        }
    }

    fn select_lexeme(&self, word: &str) -> String {
        let mut stmt = self.conn.prepare_cached(SELECT_LEXEME).unwrap();
        let init = query_statement(&mut stmt, params![word]);

        if BEGIN.eq(&init[0]) && END.eq(&init[2]) {
            return String::from(&init[1])
        }

        if BEGIN.eq(&init[0]) {
            return self.select_right(&init[1], &init[2]);
        }

        if END.eq(&init[2]) {
            return self.select_left(&init[0], &init[1], false);
        }

        // for s in &init {
        //     println!("this is case when three word are not END or BEGIN: {}", s);
        // }

        let mut result_string = self.select_left(&init[0], &init[1], true);
        result_string.push_str(" ");
        result_string.push_str(&self.select_right(&init[1], &init[2]));
        result_string
    }

    fn select_random(&self) -> String {
        let mut stmt = self.conn.prepare_cached(SELECT_BEGIN).unwrap();
        let init = query_statement(&mut stmt, params![]);

        if END.eq(&init[2]) {
            return String::from(&init[1])
        }

        // #beg# is always first
        self.select_right(&init[1], &init[2])
    }

    // maybe I can fold select_left and select_right into one universal function
    // just need to reinvent direction argument...
    fn select_left(&self, lexeme2: &str, lexeme3: &str, remove_last: bool) -> String {
        let mut stmt = self.conn.prepare_cached(SELECT_LEFT).unwrap();
        let mut result = vec![String::from(lexeme2), String::from(lexeme3)];
        let mut result_string = String::new();

        //println!("{} {}", &lexeme2, &lexeme3);

        let get_first_element = |v: &Vec<String>| v[0].clone();
        let get_after_first_element = |v: &Vec<String>| v[1].clone();

        let mut select = | word2, word3 | {
            let ret = query_statement(&mut stmt,
                                      params![&word2,
                                                      &word3]);
            if BEGIN.eq(&ret[0]) {
                None
            } else {
                Some(ret)
            }
        };

        let mut recursion = 0;
        while let Some(lexems) = select(get_first_element(&result), get_after_first_element(&result)) {
            result.insert(0, get_first_element(&lexems));
            recursion = recursion + 1;
            if recursion >= MAXIMUM_RECURSION_DEPTH {
                break
            }
        }

        // removing the last element, to make sure that it won't be duplicated in select_right
        if remove_last {
            result.pop().unwrap();
        }

        let reverse_string = |s: &str| {
            s.chars().rev().collect::<String>()
        };

        for (_i, s) in result.iter().rev().enumerate() {
            //println!("{}: {}", i, s);
            result_string.push_str(&reverse_string(&s));
            result_string.push_str(" ");
        }

        reverse_string(&result_string)
    }

    fn select_right(&self, lexeme1: &str, lexeme2: &str) -> String {
        let mut stmt = self.conn.prepare_cached(SELECT_RIGHT).unwrap();
        let mut result = vec![String::from(lexeme1), String::from(lexeme2)];
        let mut result_string = String::new();

        //println!("{} {}", &lexeme1, &lexeme2);

        let get_last_element = |v: &Vec<String>| v[v.len() - 1].clone();
        let get_prev_last_element = |v: &Vec<String>| v[v.len() - 2].clone();

        let mut select = | word1, word2 | {
            let ret = query_statement(&mut stmt,
                                         params![&word1,
                                                         &word2]);
            if END.eq(&ret[2]) {
                None
            } else {
                Some(ret)
            }
        };

        let mut recursion = 0;
        while let Some(lexems) = select(get_prev_last_element(&result), get_last_element(&result)) {
            result.push(get_last_element(&lexems));
            recursion = recursion + 1;
            if recursion >= MAXIMUM_RECURSION_DEPTH {
                break
            }
        }

        for (_i, s) in result.iter().enumerate() {
            //println!("{}: {}", i, s);
            result_string.push_str(s);
            result_string.push_str(" ");
        }

        result_string
    }
}