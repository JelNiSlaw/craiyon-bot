use std::borrow::Cow;

use tdlib::enums::{
    self, ChatMemberStatus, ChatType, InlineKeyboardButtonType, MessageContent, MessageReplyTo,
    ReplyMarkup, StickerFormat,
};
use tdlib::functions;
use tdlib::types::{
    File, FormattedText, InlineKeyboardButton, InlineKeyboardButtonTypeUrl, Message, Photo,
    ReplyMarkupInlineKeyboard, Sticker, UpdateChatMember, User, WebPage,
};

use super::cache::CompactChat;

pub trait MainUsername {
    fn main_username(&self) -> Option<&String>;
}

impl MainUsername for User {
    fn main_username(&self) -> Option<&String> {
        self.usernames.as_ref()?.active_usernames.first()
    }
}

pub struct MessageMedia {
    pub file: File,
    pub mime_type: Cow<'static, str>,
}

pub fn donate_markup(name: &str, url: impl Into<String>) -> ReplyMarkup {
    ReplyMarkup::InlineKeyboard(ReplyMarkupInlineKeyboard {
        rows: vec![vec![InlineKeyboardButton {
            text: format!("donate to {name}"),
            r#type: InlineKeyboardButtonType::Url(InlineKeyboardButtonTypeUrl { url: url.into() }),
        }]],
    })
}

pub const fn get_message_text(content: &MessageContent) -> Option<&FormattedText> {
    let formatted_text = match content {
        MessageContent::MessageText(text) => &text.text,
        MessageContent::MessageAnimation(animation) => &animation.caption,
        MessageContent::MessageAudio(audio) => &audio.caption,
        MessageContent::MessageDocument(document) => &document.caption,
        MessageContent::MessagePhoto(photo) => &photo.caption,
        MessageContent::MessageVideo(video) => &video.caption,
        MessageContent::MessageVoiceNote(voice_note) => &voice_note.caption,
        _ => return None,
    };

    Some(formatted_text)
}

pub fn get_message_image(content: &MessageContent) -> Option<MessageMedia> {
    match content {
        MessageContent::MessageText(message) => message.web_page.as_ref().and_then(web_page_image),
        MessageContent::MessageDocument(message) => Some(MessageMedia {
            file: message.document.document.clone(),
            mime_type: Cow::Owned(message.document.mime_type.clone()),
        }),
        MessageContent::MessagePhoto(message) => largest_photo(&message.photo).map(|file| {
            MessageMedia { file: file.clone(), mime_type: Cow::Borrowed("image/jpeg") }
        }),
        MessageContent::MessageSticker(message) => sticker_image(&message.sticker),
        _ => None,
    }
}

pub fn get_message_video(content: &MessageContent) -> Option<MessageMedia> {
    match content {
        MessageContent::MessageVideo(message) => Some(MessageMedia {
            file: message.video.video.clone(),
            mime_type: Cow::Borrowed("video/mp4"),
        }),
        _ => None,
    }
}

fn web_page_image(web_page: &WebPage) -> Option<MessageMedia> {
    if let Some(photo) = &web_page.photo {
        if let Some(file) = largest_photo(photo) {
            return Some(MessageMedia {
                file: file.clone(),
                mime_type: Cow::Borrowed("image/jpeg"),
            });
        }
    }

    if let Some(document) = &web_page.document {
        return Some(MessageMedia {
            file: document.document.clone(),
            mime_type: Cow::Owned(document.mime_type.clone()),
        });
    }

    if let Some(sticker) = &web_page.sticker {
        if sticker.format == StickerFormat::Webp {
            return Some(MessageMedia {
                file: sticker.sticker.clone(),
                mime_type: Cow::Borrowed("image/webp"),
            });
        }
    }

    None
}

fn largest_photo(photo: &Photo) -> Option<&File> {
    photo
        .sizes
        .iter()
        .rfind(|photo_size| photo_size.photo.local.can_be_downloaded)
        .map(|photo_size| &photo_size.photo)
}

fn sticker_image(sticker: &Sticker) -> Option<MessageMedia> {
    (sticker.format == StickerFormat::Webp)
        .then_some(MessageMedia { file: sticker.sticker.clone(), mime_type: "image/webp".into() })
}

pub async fn get_message_or_reply_image(message: &Message, client_id: i32) -> Option<MessageMedia> {
    if let Some(message_image) = get_message_image(&message.content) {
        return Some(message_image);
    }

    let MessageReplyTo::Message(reply) = message.reply_to.as_ref()? else {
        return None;
    };

    let content = if let Some(content) = reply.content.as_ref() {
        Cow::Borrowed(content)
    } else {
        let enums::Message::Message(message) =
            functions::get_replied_message(message.chat_id, message.id, client_id).await.ok()?;

        Cow::Owned(message.content)
    };

    get_message_image(&content)
}

/// Same as `get_message_or_reply_image`, but also for videos.
pub async fn get_message_or_reply_media(message: &Message, client_id: i32) -> Option<MessageMedia> {
    if let Some(message_image) = get_message_video(&message.content) {
        return Some(message_image);
    }

    if let Some(message_image) = get_message_image(&message.content) {
        return Some(message_image);
    }

    let MessageReplyTo::Message(reply) = message.reply_to.as_ref()? else {
        return None;
    };

    let content = if let Some(content) = reply.content.as_ref() {
        Cow::Borrowed(content)
    } else {
        let enums::Message::Message(message) =
            functions::get_replied_message(message.chat_id, message.id, client_id).await.ok()?;

        Cow::Owned(message.content)
    };

    match get_message_video(&content) {
        Some(message_video) => Some(message_video),
        None => get_message_image(&content),
    }
}

pub fn log_status_update(update: &UpdateChatMember, chat: &CompactChat) {
    if let ChatType::Private(_) = chat.r#type {
        return;
    }

    if update.new_chat_member.status == update.old_chat_member.status {
        return;
    }

    let status = match update.new_chat_member.status {
        ChatMemberStatus::Member => "joined",
        ChatMemberStatus::Left => "left",
        _ => return,
    };

    log::info!("{} {}", status, chat);
}
