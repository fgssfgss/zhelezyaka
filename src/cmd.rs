use log::{debug, trace};

const GENERATE_BY_WORD_COMMAND: &str = "/q";
const DISABLE_FOR_CHAT_COMMAND: &str = "/off";
const ENABLE_FOR_CHAT_COMMAND: &str = "/on";
const GET_WORD_COUNT_COMMAND: &str = "/count";
const START_BOT_COMMAND: &str = "/start";

pub struct CommandParser;

pub enum CommandType {
    ENoCommand,
    EGenerateByWord(String),
    EGetCountByWord(String),
    EDisableForChat,
    EEnableForChat,
    ENewUser,
}

impl CommandParser {
    pub fn parse_command(input: &String) -> CommandType {
        let tokens: Vec<&str> = input.trim().split_whitespace().collect();

        if tokens.is_empty() {
            debug!("tokens vec is empty");
            return CommandType::ENoCommand
        }

        match tokens[0] {
            START_BOT_COMMAND => {
                trace!("New user started bot");
                CommandType::ENewUser
            },
            GENERATE_BY_WORD_COMMAND => {
                if tokens.len() == 2 {
                    trace!("Gen by word");
                    CommandType::EGenerateByWord(String::from(tokens[1]))
                } else {
                    trace!("Gen by word: nocmd here");
                    CommandType::ENoCommand
                }
            },
            GET_WORD_COUNT_COMMAND => {
                if tokens.len() == 2 {
                    CommandType::EGetCountByWord(String::from(tokens[1]))
                } else {
                    CommandType::ENoCommand
                }
            },
            DISABLE_FOR_CHAT_COMMAND => CommandType::EDisableForChat,
            ENABLE_FOR_CHAT_COMMAND => CommandType::EEnableForChat,
            _ => CommandType::ENoCommand
        }
    }
}