use crate::user::*;
use crate::sqlite::*;
use std::collections::HashMap;
use std::sync::{Mutex, Arc};

pub struct UserManager {
    user_table: Arc<Mutex<HashMap<String, UserAccount>>>,
}

impl UserManager {
    pub fn new(conn: &SqliteConn) -> UserManager
    {
        UserManager { user_table: Arc::new(Mutex::new(conn.get_all_users())) }
    }

    pub fn get_user(&self, conn: &SqliteConn, user_id: &str) -> UserAccount {
        UserManager::get_or_insert(Arc::clone(&self.user_table), conn, user_id)
    }

    pub fn update_user(&self, conn: &SqliteConn, user: &UserAccount) {
        UserManager::update_or_ignore(Arc::clone(&self.user_table), conn, user)
    }

    fn insert_account(map: &mut HashMap<String, UserAccount>, conn: &SqliteConn, user_id: &str) {
        let user_account = UserAccount { user_id: String::from(user_id), is_admin: false, answer_mode: true};
        conn.insert_user(&user_account);
        map.insert(String::from(user_id), user_account);
    }

    fn get_or_insert(table: Arc<Mutex<HashMap<String, UserAccount>>>, conn: &SqliteConn, user_id: &str) -> UserAccount {
        let locked_table = &table;
        let mut hash_table = locked_table.lock().unwrap();

        if !hash_table.contains_key(user_id) {
            UserManager::insert_account(&mut hash_table, conn, user_id);
        }

        // previously we inserted it in sqlite and in hashmap
        hash_table.get(user_id).unwrap().clone()
    }

    fn update_or_ignore(table: Arc<Mutex<HashMap<String, UserAccount>>>, conn: &SqliteConn, user: &UserAccount) {
        let locked_table = &table;
        let mut hash_table = locked_table.lock().unwrap();

        if hash_table.contains_key(&user.user_id) {
            hash_table.insert(user.user_id.clone(), user.clone());
            conn.update_user(user);
        }
    }
}