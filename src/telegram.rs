use futures::StreamExt;
use telegram_bot::*;
use log::{debug, trace, info, error };

pub struct Telegram {
    api: Api
}

pub enum TelegramActions {
    ReplyToMessage(String),
    ReplyToChat(String),
    NoReply
}

impl Telegram {
    pub fn new(token: &str) -> Telegram {
        let api = Api::new(token);
        debug!("Creating a telegram interface");
        Telegram { api }
    }

    async fn send_reply(api: Api, message: &Message, text: String) {
        trace!("Sending a reply");
        while let Err(e) = api.send(message.text_reply(&text)).await {
            error!("Error in api.send_reply() - {}", e);
        }
    }

    async fn send_to_chat(api: Api, message: &Message, text: String) {
        trace!("Sending to a chat");
        while let Err(e) = api.send(message.chat.text(&text)).await {
            error!("Error in api.send_to_chat() - {}", e);
        }
    }

    async fn send_message(api: Api, message: &Message, action: TelegramActions) {
        match action {
            TelegramActions::ReplyToMessage(s) => Telegram::send_reply(api, message, s).await,
            TelegramActions::ReplyToChat(s) => Telegram::send_to_chat(api, message, s).await,
            TelegramActions::NoReply => { trace!("No reply to this command"); }
        };
    }

    pub async fn serve<F>(&self, func: F) -> ()
    where
        F: Fn(ChatId, String) -> TelegramActions,
        F: Copy + Send + 'static,
    {
        let mut stream = self.api.stream();
        while let Some(update) = stream.next().await {
            let update = update.unwrap();
            if let UpdateKind::Message(message) = update.kind {
                if let MessageKind::Text { ref data, .. } = message.kind {
                    let api = self.api.clone();
                    let data = data.clone();
                    let chat_id = message.chat.id();
                    tokio::spawn(async move {
                        info!("<{}>: {}", &message.from.first_name, data);
                        let action = tokio::task::block_in_place(move || {
                            func(chat_id, data)
                        });
                        Telegram::send_message(api, &message, action).await;
                    });
                }
            }
        }
    }
}
