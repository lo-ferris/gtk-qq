mod chatroom;
mod sidebar;

use std::cell::RefCell;
use std::collections::VecDeque;

use once_cell::sync::OnceCell;
use relm4::factory::FactoryVecDeque;
use relm4::{
    adw, gtk, Component, ComponentController, ComponentParts, ComponentSender, Controller,
    SimpleComponent,
};

use adw::{prelude::*, HeaderBar, Leaflet};
use gtk::{Align, Box, Label, MenuButton, Orientation, Separator, Stack};

use crate::handler::{ACCOUNT, FRIEND_LIST};
use chatroom::{Chatroom, ChatroomInitParams};
pub use sidebar::ContactGroup;
use sidebar::{SidebarModel, SidebarMsg};

pub static MAIN_SENDER: OnceCell<ComponentSender<MainPageModel>> = OnceCell::new();

#[derive(Debug)]
pub struct MainPageModel {
    message: Option<ViewMsg>,
    sidebar: Controller<SidebarModel>,
    chatrooms: RefCell<FactoryVecDeque<Stack, Chatroom, MainMsg>>,
}

impl MainPageModel {
    fn is_item_in_list(&self, account: i64, is_group: bool) -> bool {
        let chatrooms = self.chatrooms.borrow();

        for i in 0..chatrooms.len() {
            let chatroom = chatrooms.get(i);
            if chatroom.account == account && chatroom.is_group == is_group {
                return true;
            }
        }

        false
    }

    fn insert_chatroom(&self, account: i64, is_group: bool) {
        // TODO: Get history messages
        let messages = VecDeque::new();
        let mut chatrooms = self.chatrooms.borrow_mut();
        chatrooms.push_front(ChatroomInitParams {
            account,
            is_group,
            messages,
        });
    }

    fn push_own_friend_message(&self, target: i64, content: String) {
        let self_account = *ACCOUNT.get().unwrap();
        let mut chatrooms = self.chatrooms.borrow_mut();
        for i in 0..chatrooms.len() {
            let mut chatroom = chatrooms.get_mut(i);
            if chatroom.account == target && !chatroom.is_group {
                chatroom.push_message(Message {
                    sender: self_account,
                    target,
                    content,
                });
                break;
            }
        }
    }

    fn push_others_message(&self, group: i64, is_group: bool, sender: i64, content: String) {
        let self_account = *ACCOUNT.get().unwrap();
        let mut chatrooms = self.chatrooms.borrow_mut();
        for i in 0..chatrooms.len() {
            let mut chatroom = chatrooms.get_mut(i);
            if chatroom.account == group && chatroom.is_group == is_group {
                chatroom.push_message(Message {
                    sender,
                    target: self_account,
                    content,
                });
                break;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    pub sender: i64,
    pub target: i64,
    pub content: String,
}

#[derive(Debug)]
pub enum MainMsg {
    WindowFolded,
    /// 对应侧边栏中的条目ID（群号/QQ号）,
    /// 是否为群组消息,
    /// sender_id,
    /// content
    ReceiveMessage(i64, bool, i64, String),
    SendFriendMessage(i64, String),
    SelectChatroom(i64, bool),
    InitSidebar,
}

#[derive(Debug)]
enum ViewMsg {
    WindowFolded,
    SelectChatroom(i64, bool),
}

relm4::new_action_group!(WindowActionGroup, "menu");
relm4::new_stateless_action!(ShortcutsAction, WindowActionGroup, "shortcuts");
relm4::new_stateless_action!(AboutAction, WindowActionGroup, "about");

#[relm4::component(pub)]
impl SimpleComponent for MainPageModel {
    type Input = MainMsg;
    type Output = ();
    type Widgets = MainPageWidgets;
    type InitParams = ();

    view! {
        #[root]
        main_page = &Leaflet {
            append: sidebar_controller.widget(),
            append = &Separator::new(Orientation::Horizontal),
            append: chatroom = &Box {
                set_vexpand: true,
                set_hexpand: true,
                set_orientation: Orientation::Vertical,
                append = &HeaderBar {
                    set_title_widget = Some(&Box) {
                        set_orientation: Orientation::Vertical,
                        set_valign: Align::Center,
                        append: chatroom_title = &Label {
                            set_label: "Chatroom"
                        },
                        append: chatroom_subtitle = &Label {
                            set_css_classes: &["subtitle"],
                            set_label: "Chatroom"
                        },
                    },
                    pack_end = &MenuButton {
                        set_icon_name: "menu-symbolic",
                        set_menu_model: Some(&main_menu),
                    }
                },
                append: chatroom_stack = &Stack {},
            },
            connect_folded_notify[sender] => move |leaflet| {
                if leaflet.is_folded() {
                    sender.input(MainMsg::WindowFolded);
                }
            },
        }
    }

    menu! {
        main_menu: {
            "Keyboard Shortcuts" => ShortcutsAction,
            "About Gtk QQ" => AboutAction
        }
    }

    fn init(
        _init_params: Self::InitParams,
        root: &Self::Root,
        sender: &ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        MAIN_SENDER
            .set(sender.clone())
            .expect("failed to initialize main sender");

        let sidebar_controller = SidebarModel::builder()
            .launch(())
            .forward(&sender.input, |message| message);

        let widgets = view_output!();

        let chatrooms: FactoryVecDeque<Stack, Chatroom, MainMsg> =
            FactoryVecDeque::new(widgets.chatroom_stack.clone(), &sender.input);

        ComponentParts {
            model: MainPageModel {
                message: None,
                sidebar: sidebar_controller,
                chatrooms: RefCell::new(chatrooms),
            },
            widgets,
        }
    }

    fn update(&mut self, msg: MainMsg, _sender: &ComponentSender<Self>) {
        use MainMsg::*;
        match msg {
            WindowFolded => self.message = Some(ViewMsg::WindowFolded),
            SelectChatroom(account, is_group) => {
                if !self.is_item_in_list(account, is_group) {
                    // TODO: Get last_message from history or some other places
                    self.sidebar.sender().send(SidebarMsg::InsertChatItem(
                        account,
                        false,
                        String::new(),
                    ));
                    self.insert_chatroom(account, is_group)
                }

                self.message = Some(ViewMsg::SelectChatroom(account, is_group));
            }
            SendFriendMessage(target, content) => {
                use SidebarMsg::*;
                if self.is_item_in_list(target, false) {
                    self.sidebar
                        .sender()
                        .send(UpdateChatItem(target, false, content.clone()));
                } else {
                    self.sidebar
                        .sender()
                        .send(InsertChatItem(target, false, content.clone()));
                    self.insert_chatroom(target, false);
                    // 当所插入的 chatroom 为唯一的一个 chatroom 时，将其设为焦点，
                    // 以触发自动更新 chatroom 的标题与副标题。
                    if self.chatrooms.borrow().len() == 1 {
                        self.message = Some(ViewMsg::SelectChatroom(target, false));
                    }
                }
                self.push_own_friend_message(target, content);
            }
            ReceiveMessage(chat_item, is_group, sender, content) => {
                use SidebarMsg::*;
                if self.is_item_in_list(chat_item, is_group) {
                    self.sidebar.sender().send(UpdateChatItem(
                        chat_item,
                        is_group,
                        content.clone(),
                    ));
                } else {
                    self.sidebar.sender().send(InsertChatItem(
                        chat_item,
                        is_group,
                        content.clone(),
                    ));
                    self.insert_chatroom(chat_item, is_group);
                    // 当所插入的 chatroom 为唯一的一个 chatroom 时，将其设为焦点，
                    // 以触发自动更新 chatroom 的标题与副标题。
                    if self.chatrooms.borrow().len() == 1 {
                        self.message = Some(ViewMsg::SelectChatroom(chat_item, is_group));
                    }
                }

                self.push_others_message(chat_item, is_group, sender, content);
            }
            InitSidebar => {
                self.sidebar.sender().send(SidebarMsg::RefreshContact);
            }
        }
        self.chatrooms.borrow().render_changes();
    }

    fn pre_view() {
        if let Some(message) = &model.message {
            use ViewMsg::*;
            match message {
                WindowFolded => widgets.main_page.set_visible_child(&widgets.chatroom),
                SelectChatroom(account, is_group) => {
                    let child_name =
                        &format!("{} {}", account, if *is_group { "group" } else { "friend" });
                    chatroom_stack.set_visible_child_name(child_name);
                    if *is_group {
                        let title = format!("{}", account);
                        let subtitle = format!("{}", account);
                        chatroom_title.set_label(&title);
                        chatroom_subtitle.set_label(&subtitle);
                    } else {
                        let user = FRIEND_LIST
                            .get()
                            .unwrap()
                            .iter()
                            .find(|user| user.uin == *account)
                            .unwrap();
                        let title = &user.remark;
                        let subtitle = format!("{} ({})", user.nick, account);
                        chatroom_title.set_label(title);
                        chatroom_subtitle.set_label(&subtitle);
                    }
                }
            }
        }
    }
}