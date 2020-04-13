use futures::StreamExt;
use telegram_bot::*;
use log::{debug, trace, info};

pub struct Telegram{
    api: Api,
    stream: UpdatesStream,
}

impl Telegram {
    pub fn new(token: &str) -> Telegram {
        let api = Api::new(token);
        let stream = api.stream();
        debug!("Creating a telegram interface");
        Telegram { api, stream }
    }

    // TODO: Make it not reply
    pub async fn send_message(&self, message: &Message, text: String) -> () {
        trace!("Sending a reply");
        // TODO: Handle error
        self.api.send(message.text_reply(text)).await;
    }

    pub async fn serve<F>(&mut self, mut func: F) -> ()
    where
        F: FnMut(&String) -> String,
    {
        while let Some(update) = self.stream.next().await {
            let update = update.unwrap();
            if let UpdateKind::Message(message) = update.kind {
                if let MessageKind::Text { ref data, .. } = message.kind {
                    info!("<{}>: {}", &message.from.first_name, data);
                    // TODO: function should return something
                    self.send_message(&message, func(data)).await;
                }
            }
        }
    }
}
