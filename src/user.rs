#[derive(Debug, Clone)]
pub struct UserAccount {
    pub user_id: String,
    pub is_admin: bool,
    pub answer_mode: bool,
}