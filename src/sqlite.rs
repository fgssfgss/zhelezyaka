use log::{debug, info, trace};
use r2d2_sqlite::SqliteConnectionManager;
use r2d2_sqlite::rusqlite::params;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::rusqlite::*;
use r2d2_sqlite::rusqlite::ToSql;
use std::collections::HashMap;
use crate::user::*;

const CREATE_DB: &str = "CREATE TABLE IF NOT EXISTS lexems (\
                            `lexeme1` TEXT, \
                            `lexeme2` TEXT, \
                            `lexeme3` TEXT, \
                            `count` INT NOT NULL DEFAULT '0', \
                            UNIQUE (`lexeme1`, `lexeme2`, `lexeme3`));";
const CREATE_USER_DB: &str = "CREATE TABLE IF NOT EXISTS user_profiles (\
                            `user_id` TEXT,\
                            `is_admin` INT NOT NULL DEFAULT '0',\
                            `answer_mode` INT NOT NULL DEFAULT '1',\
                            UNIQUE (`user_id`));";
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
    pool: Pool<SqliteConnectionManager>
}

impl SqliteDB {
    pub fn new(path: &str) -> SqliteDB {
        info!("SqliteDB starting");
        let manager = SqliteConnectionManager::file(path);
        let pool = r2d2::Pool::new(manager).unwrap();


        let conn = pool.get().unwrap();
        conn.execute(CREATE_DB, params![]).unwrap();
        conn.execute(CREATE_USER_DB, params![]).unwrap();
        SqliteDB { pool }
    }

    pub fn get_conn(&self) -> SqliteConn {
        SqliteConn::new(self.pool.get().unwrap())
    }
}

pub struct SqliteConn {
    conn: PooledConnection<SqliteConnectionManager>
}

fn query_statement<P>(stmt: &mut CachedStatement<'_>, params: P) -> Vec<String>
where
    P: IntoIterator,
    P::Item: ToSql,
{
    match stmt.query_row(params, |row| {
        let lexeme1: String = row.get_unwrap(0);
        let lexeme2: String = row.get_unwrap(1);
        let lexeme3: String = row.get_unwrap(2);
        Ok(vec![lexeme1, lexeme2, lexeme3])
    }) {
        Ok(v) => v,
        Err(_) => return vec![String::from(BEGIN),
                                     String::from("Not found"),
                                     String::from(END)],
    }
}

impl SqliteConn {
    pub fn new(conn: PooledConnection<SqliteConnectionManager>) -> SqliteConn {
        conn.busy_handler(Some(|_| {
            std::thread::sleep(std::time::Duration::from_millis(16));
            true
        })).unwrap();
        SqliteConn { conn }
    }

    pub fn is_exist(&self, word: String) -> Option<i32> {
        if !word.is_empty() {
            let mut stmt = self.conn.prepare_cached(IS_EXIST).unwrap();
            let count = stmt.query_and_then(params![&word], |row| {
                let cnt: i32 = row.get_unwrap(0);
                Ok(cnt)
            }).unwrap().
                fold(0, |a, b: std::result::Result<i32, Error>| a + b.unwrap());

            Some(count)
        } else {
            None
        }
    }

    pub fn insert(&self, text: String) -> () {
        let mut splitted: Vec<&str> = text.trim().split_whitespace().collect();

        splitted.insert(0, BEGIN);
        splitted.push(END);

        for s in &splitted {
            trace!("{}", s);
        }

        self.conn.execute("BEGIN DEFERRED TRANSACTION", params![]).unwrap();
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

            trace!("Inserting {}", insert_sql);
            match self.conn.execute(&insert_sql, params![]) {
                Err(e) => trace!("Insert was unsuccessful: {}", e),
                _ => trace!("Insert is successful"),
            }

            trace!("Inserting {}", update_sql);
            match self.conn.execute(&update_sql, params![]) {
                Err(e) => trace!("Insert was unsuccessful: {}", e),
                _ => trace!("Insert is successful"),
            }
        }
        self.conn.execute("COMMIT", params![]).unwrap();
    }

    pub fn select(&self, input: String) -> String {
        let word= input.trim();

        if word.is_empty() {
            let result = self.select_random();
            debug!("Found by random: {}", result);
            result
        } else {
            let result = self.select_lexeme(word);
            debug!("Found by word {}: {}", word, result);
            result
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

        trace!("{} {}", &lexeme2, &lexeme3);

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

        for (i, s) in result.iter().rev().enumerate() {
            trace!("{}: {}", i, s);
            result_string.push_str(&reverse_string(&s));
            result_string.push_str(" ");
        }

        reverse_string(&result_string)
    }

    fn select_right(&self, lexeme1: &str, lexeme2: &str) -> String {
        let mut stmt = self.conn.prepare_cached(SELECT_RIGHT).unwrap();
        let mut result = vec![String::from(lexeme1), String::from(lexeme2)];
        let mut result_string = String::new();

        trace!("{} {}", &lexeme1, &lexeme2);

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

        for (i, s) in result.iter().enumerate() {
            trace!("{}: {}", i, s);
            result_string.push_str(s);
            result_string.push_str(" ");
        }

        result_string
    }

    // API for user management
    // So I will use only insert user into this DB
    pub fn insert_user(&self, user: &UserAccount) {
        self.conn.execute("BEGIN DEFERRED TRANSACTION", params![]).unwrap();

        let query = "INSERT OR IGNORE INTO user_profiles (`user_id`, `is_admin`, `answer_mode`) VALUES (?1, ?2, ?3)";
        self.conn.execute(&query, params![&user.user_id, user.is_admin, user.answer_mode]).unwrap();

        self.conn.execute("COMMIT", params![]).unwrap();
    }

    // I will use it only on startup
    pub fn get_all_users(&self) -> HashMap<String, UserAccount> {
        let mut map = HashMap::new();
        let mut stmt = self.conn.prepare_cached("SELECT * FROM user_profiles").unwrap();
        let _n = stmt.query_and_then(params![], |row| {
            let user_id: String = row.get_unwrap(0);
            let is_admin: bool = row.get_unwrap(1);
            let answer_mode: bool = row.get_unwrap(2);

            info!("fetching profile = {} {} {}", &user_id, is_admin, answer_mode);
            Ok(UserAccount { user_id, is_admin, answer_mode })
        }).unwrap().map(|item: Result<UserAccount, Error>| {
            let user = item.unwrap();
            map.insert(user.user_id.clone(), user);
        }).collect::<()>();
        map
    }

    pub fn update_user(&self, user: &UserAccount) {
        self.conn.execute("BEGIN DEFERRED TRANSACTION", params![]).unwrap();

        let query = "UPDATE user_profiles SET `is_admin` = ?2, `answer_mode` = ?3 WHERE user_id = '?1'";
        self.conn.execute(&query, params![&user.user_id, user.is_admin, user.answer_mode]).unwrap();

        self.conn.execute("COMMIT", params![]).unwrap();
    }
}