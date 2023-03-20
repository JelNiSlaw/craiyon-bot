use std::sync::{Arc, Mutex};

use tdlib::enums::{ChatType, MessageContent, MessageSender, UserType};
use tdlib::types::{
    ChatPermissions, ChatTypeSupergroup, FormattedText, Message, MessageSenderUser, MessageText,
};

use super::cache::{CompactChat, CompactUser};
use super::command_context::CommandContext;
use super::message_queue::MessageQueue;
use super::rate_limit::{RateLimiter, RateLimits};

pub fn command_context() -> CommandContext {
    CommandContext {
        chat: CompactChat {
            r#type: ChatType::Supergroup(ChatTypeSupergroup::default()),
            title: "chat_title".into(),
            permissions: ChatPermissions::default(),
        },
        user: CompactUser {
            id: 0,
            first_name: "user_first_name".into(),
            last_name: "user_last_name".into(),
            username: Some("user_username".into()),
            r#type: UserType::Regular,
            language_code: "user_language_code".into(),
        },
        message: Message {
            id: 0,
            sender_id: MessageSender::User(MessageSenderUser::default()),
            chat_id: 0,
            sending_state: None,
            scheduling_state: None,
            is_outgoing: false,
            is_pinned: false,
            can_be_edited: false,
            can_be_forwarded: false,
            can_be_saved: false,
            can_be_deleted_only_for_self: false,
            can_be_deleted_for_all_users: false,
            can_get_added_reactions: false,
            can_get_statistics: false,
            can_get_message_thread: false,
            can_get_viewers: false,
            can_get_media_timestamp_links: false,
            can_report_reactions: false,
            has_timestamped_media: false,
            is_channel_post: false,
            is_topic_message: false,
            contains_unread_mention: false,
            date: 0,
            edit_date: 0,
            forward_info: None,
            interaction_info: None,
            unread_reactions: Vec::new(),
            reply_in_chat_id: 0,
            reply_to_message_id: 0,
            message_thread_id: 0,
            self_destruct_time: 0,
            self_destruct_in: 0.,
            auto_delete_in: 0.,
            via_bot_user_id: 0,
            author_signature: "message_author_signature".into(),
            media_album_id: 0,
            restriction_reason: "message_restriction_reason".into(),
            content: MessageContent::MessageText(MessageText {
                text: FormattedText { text: "message_content_text".into(), entities: Vec::new() },
                web_page: None,
            }),
            reply_markup: None,
        },
        client_id: 0,
        rate_limits: Arc::new(Mutex::new(RateLimits {
            rate_limit_exceeded: RateLimiter::new(0, 0),
        })),
        message_queue: Arc::new(MessageQueue::default()),
        http_client: reqwest::Client::new(),
    }
}
