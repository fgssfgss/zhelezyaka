use futures::StreamExt;
use telegram_bot::*;
use log::{debug, trace, info, error };

pub struct Telegram {
    api: Api,
    stream: UpdatesStream,
}

pub enum TelegramActions {
    ReplyToMessage(String),
    ReplyToChat(String),
    NoReply
}

impl Telegram {
    pub fn new(token: &str) -> Telegram {
        let api = Api::new(token);
        let stream = api.stream();
        debug!("Creating a telegram interface");
        Telegram { api, stream }
    }

    async fn send_reply(&self, message: &Message, text: String) {
        trace!("Sending a reply");
        while let Err(e) = self.api.send(message.text_reply(&text)).await {
            error!("Error in api.send_reply() - {}", e);
        }
    }

    async fn send_to_chat(&self, message: &Message, text: String) {
        trace!("Sending to a chat");
        while let Err(e) = self.api.send(message.chat.text(&text)).await {
            error!("Error in api.send_to_chat() - {}", e);
        }
    }

    async fn send_message(&self, message: &Message, action: TelegramActions) {
        match action {
            TelegramActions::ReplyToMessage(s) => self.send_reply(message, s).await,
            TelegramActions::ReplyToChat(s) => self.send_to_chat(message, s).await,
            TelegramActions::NoReply => { trace!("No reply to this command"); }
        };
    }

    pub async fn serve<F>(&mut self, mut func: F) -> ()
    where
        F: FnMut(ChatId, &String) -> TelegramActions,
    {
        while let Some(update) = self.stream.next().await {
            let update = update.unwrap();
            if let UpdateKind::Message(message) = update.kind {
                if let MessageKind::Text { ref data, .. } = message.kind {
                    info!("<{}>: {}", &message.from.first_name, data);
                    self.send_message(&message, func(message.chat.id(), data)).await;
                }
            }
        }
    }
}
