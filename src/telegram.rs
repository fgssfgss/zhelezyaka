use futures::StreamExt;
use telegram_bot::*;
use log::{debug, trace, info, error, warn};
use std::path::Path;
use std::ffi::OsStr;

macro_rules! make_reply {
    ($e:expr) => (TelegramActions::ReplyToMessage(String::from($e)));
}

const FILE_SIZE_LIMIT_BYTES: i64 = 1_000_000; // 1 MiB

pub struct Telegram {
    api: Api,
    token: String
}

#[derive(Debug)]
enum TelegramErrors {
    FileSizeIsTooBig,
    FileExtensionMissingOrWrong,
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
        Telegram { api, token: String::from(token) }
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

    fn get_extension_from_filename(filename: &str) -> Option<&str> {
        Path::new(filename)
            .extension()
            .and_then(OsStr::to_str)
    }

    async fn validate_and_get_document_url(token: String, api: &Api, document: Document) -> Result<String, TelegramErrors> {
        let link = api.send(GetFile::new(&document)).await.unwrap();
        info!("filesize {}", link.file_size.unwrap());
        if link.file_size.unwrap_or(FILE_SIZE_LIMIT_BYTES) >= FILE_SIZE_LIMIT_BYTES {
            return Err(TelegramErrors::FileSizeIsTooBig)
        }
        let file_name = link.file_path.unwrap();
        if !Telegram::get_extension_from_filename(&file_name).map_or(false, |ext| ext == "txt") {
            return Err(TelegramErrors::FileExtensionMissingOrWrong)
        }
        let url = format!("https://api.telegram.org/file/bot{}/{}", token, file_name);
        Ok(url)
    }

    async fn download_document_from_url(url: String) -> String {
        let response = reqwest::get(&url)
            .await.unwrap();

        for (key, value) in response.headers().iter() {
            println!("HEADERS {:?}: {:?}", key, value);
        };

        let body = response.text()
            .await.unwrap();

        println!("body = {:?}", body);
        body
    }

    pub async fn serve<F, P>(&self, message_handler: F, file_handler: P) -> ()
    where
        F: Fn(ChatId, String) -> TelegramActions,
        F: Copy + Send + 'static,
        P: Fn(String),
        P: Copy + Send + 'static,
    {
        let mut stream = self.api.stream();
        while let Some(update) = stream.next().await {
            let update = update.unwrap();
            if let UpdateKind::Message(message) = update.kind {
                 match message.kind {
                     MessageKind::Text { ref data, .. } => {
                         let api = self.api.clone();
                         let data = data.clone();
                         let chat_id = message.chat.id();
                         tokio::spawn(async move {
                             info!("<{}>: {}", &message.from.first_name, data);
                             let action = tokio::task::block_in_place(move || {
                                 message_handler(chat_id, data)
                             });
                             Telegram::send_message(api, &message, action).await;
                         });
                     },
                     MessageKind::Document { ref data, .. } => {
                         // save the document, parse it as .txt file and push data into sqlite
                         let api = self.api.clone();
                         let token = self.token.clone();
                         let document = data.clone();
                         tokio::spawn(async move {
                             let doc = Telegram::validate_and_get_document_url(token, &api, document).await;
                             match doc {
                                 Ok(url) => {
                                     Telegram::send_message(api, &message, make_reply!("File is in progress")).await;
                                     info!("document {}", url);
                                     let file_body = Telegram::download_document_from_url(url).await;
                                     file_handler(file_body);
                                 },
                                 Err(e) => {
                                     warn!("Error in file validation: {:?}", e);
                                     Telegram::send_message(api, &message, make_reply!("Some problems with file, maybe it's too big, or it's not a txt file")).await;
                                 }
                             };
                         });
                     },
                     typ => {
                         warn!("Unknown message type {:?}", typ);
                     }
                 }
            }
        }
    }
}
