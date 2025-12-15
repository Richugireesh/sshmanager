use crate::config::{Server, AuthType};
use ratatui::widgets::ListState;
use tui_textarea::TextArea;

pub enum InputMode {
    Normal,
    Editing,
}

pub enum Focus {
    ServerList,
    Form(FormFocus),
}

#[derive(Clone, Copy)]
pub enum FormFocus {
    Group,
    Name,
    User,
    Host,
    Port,
    AuthType,
    PasswordOrKey,
    Submit,
}

impl FormFocus {
    fn next(&self) -> Self {
        match self {
            Self::Group => Self::Name,
            Self::Name => Self::User,
            Self::User => Self::Host,
            Self::Host => Self::Port,
            Self::Port => Self::AuthType,
            Self::AuthType => Self::PasswordOrKey,
            Self::PasswordOrKey => Self::Submit,
            Self::Submit => Self::Group,
        }
    }
}

pub struct App<'a> {
    pub servers: Vec<Server>,
    pub list_state: ListState,
    pub show_popup: bool,
    pub input_mode: InputMode,
    pub focus: Focus,
    
    // Form Inputs
    pub group_input: TextArea<'a>,
    pub name_input: TextArea<'a>,
    pub user_input: TextArea<'a>,
    pub host_input: TextArea<'a>,
    pub port_input: TextArea<'a>,
    pub auth_type_idx: usize,
    pub password_key_input: TextArea<'a>,

    pub should_quit: bool,
    pub should_connect: Option<usize>, // Index of server to connect to
}

impl<'a> App<'a> {
    pub fn new(servers: Vec<Server>) -> App<'a> {
        let mut list_state = ListState::default();
        if !servers.is_empty() {
            list_state.select(Some(0));
        }

        App {
            servers,
            list_state,
            show_popup: false,
            input_mode: InputMode::Normal,
            focus: Focus::ServerList,
            
            group_input: TextArea::default(),
            name_input: TextArea::default(),
            user_input: TextArea::default(),
            host_input: TextArea::default(),
            port_input: TextArea::default(),
            auth_type_idx: 0, 
            password_key_input: TextArea::default(),

            should_quit: false,
            should_connect: None,
        }
    }

    pub fn on_tick(&mut self) {}

    pub fn next(&mut self) {
        if self.servers.is_empty() { return; }
        
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.servers.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.servers.is_empty() { return; }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.servers.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn open_add_server_popup(&mut self) {
        self.show_popup = true;
        self.input_mode = InputMode::Editing;
        self.focus = Focus::Form(FormFocus::Group);
        
        // Reset inputs
        self.group_input = TextArea::from(vec!["General"]);
        self.name_input = TextArea::default();
        self.user_input = TextArea::default();
        self.host_input = TextArea::default();
        self.port_input = TextArea::from(vec!["22"]);
        self.password_key_input = TextArea::default();
        self.auth_type_idx = 0;
    }

    pub fn close_popup(&mut self) {
        self.show_popup = false;
        self.input_mode = InputMode::Normal;
        self.focus = Focus::ServerList;
    }

    pub fn next_form_field(&mut self) {
        if let Focus::Form(current) = &self.focus {
            self.focus = Focus::Form(current.next());
        }
    }

    pub fn save_server(&mut self) {
        let port = self.port_input.lines()[0].parse().unwrap_or(22);
        
        // Map index to AuthType
        let auth = match self.auth_type_idx {
            0 => AuthType::Password(self.password_key_input.lines()[0].to_string()),
            1 => AuthType::Key(self.password_key_input.lines()[0].to_string()),
            _ => AuthType::Agent,
        };

        let server = Server {
            group: self.group_input.lines()[0].to_string(),
            name: self.name_input.lines()[0].to_string(),
            user: self.user_input.lines()[0].to_string(),
            host: self.host_input.lines()[0].to_string(),
            port,
            auth_type: auth,
        };

        self.servers.push(server);
        self.close_popup();
    }
}
