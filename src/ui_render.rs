use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::app::{App, Focus, FormFocus, InputMode};
use crate::config::AuthType;

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)]) // Main + Footer
        .split(f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[0]);

    // SERVER LIST
    let items: Vec<ListItem> = app.servers
        .iter()
        .map(|s| {
            let content = vec![Line::from(Span::raw(format!("{} [{}]", s.name, s.group)))];
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Servers"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, main_chunks[0], &mut app.list_state);

    // SERVER DETAILS
    let detail_text = if let Some(idx) = app.list_state.selected() {
        if idx < app.servers.len() {
            let s = &app.servers[idx];
            let auth = match &s.auth_type {
                AuthType::Password(_) => "Password",
                AuthType::Key(_) => "SSH Key",
                AuthType::Agent => "SSH Agent",
            };
            
            vec![
                Line::from(vec![Span::raw("Alias: "), Span::styled(&s.name, Style::default().fg(Color::Cyan))]),
                Line::from(vec![Span::raw("Group: "), Span::styled(&s.group, Style::default().fg(Color::Green))]),
                Line::from(""),
                Line::from(vec![Span::raw("Host: "), Span::styled(&s.host, Style::default().fg(Color::Yellow))]),
                Line::from(vec![Span::raw("User: "), Span::styled(&s.user, Style::default().fg(Color::Yellow))]),
                Line::from(vec![Span::raw("Port: "), Span::styled(s.port.to_string(), Style::default().fg(Color::Yellow))]),
                Line::from(""),
                Line::from(vec![Span::raw("Auth Method: "), Span::styled(auth, Style::default().fg(Color::Magenta))]),
            ]
        } else {
            vec![Line::from("No server selected")]
        }
    } else {
        vec![Line::from("Select a server")]
    };

    let details = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .wrap(Wrap { trim: true });

    f.render_widget(details, main_chunks[1]);

    // FOOTER HELP
    let msg = match app.input_mode {
        InputMode::Normal => "q: Quit | a: Add | Enter: Connect | t: SFTP | j/k: Nav",
        InputMode::Editing => "Esc: Cancel | Tab: Next Field | Enter: Submit",
    };
    let footer = Paragraph::new(msg).style(Style::default().bg(Color::Blue).fg(Color::White));
    f.render_widget(footer, chunks[1]);

    // POPUP FORM
    if app.show_popup {
        let block = Block::default().title("Add Server").borders(Borders::ALL).style(Style::default().bg(Color::DarkGray));
        let area = centered_rect(60, 60, f.area());
        f.render_widget(Clear, area); // Clear background
        f.render_widget(block, area);

        let form_layout = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Group
                Constraint::Length(3), // Name
                Constraint::Length(3), // User
                Constraint::Length(3), // Host
                Constraint::Length(3), // Port
                Constraint::Length(3), // Auth Type
                Constraint::Length(3), // Pass/Key
                Constraint::Length(3), // Submit
            ])
            .split(area);

        // Helper to render input
        let draw_input = |f: &mut Frame, widget: &tui_textarea::TextArea, rect: Rect, focus: bool| {
            let mut w = widget.clone();
            if focus {
                 w.set_style(Style::default().fg(Color::Yellow));
                 w.set_block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Yellow)));
            } else {
                 w.set_block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::White)));
            }
            f.render_widget(w.widget(), rect);
        };

        let current_focus = if let Focus::Form(f) = &app.focus { Some(*f) } else { None };

        draw_input(f, &app.group_input, form_layout[0], matches!(current_focus, Some(FormFocus::Group)));
        draw_input(f, &app.name_input, form_layout[1], matches!(current_focus, Some(FormFocus::Name)));
        draw_input(f, &app.user_input, form_layout[2], matches!(current_focus, Some(FormFocus::User)));
        draw_input(f, &app.host_input, form_layout[3], matches!(current_focus, Some(FormFocus::Host)));
        draw_input(f, &app.port_input, form_layout[4], matches!(current_focus, Some(FormFocus::Port)));

        // Auth Type Selector
        let auth_options = vec!["Password", "Key", "Agent"];
        let auth_text = format!("Auth Type: < {} >", auth_options[app.auth_type_idx]);
        let auth_style = if matches!(current_focus, Some(FormFocus::AuthType)) { Style::default().fg(Color::Yellow) } else { Style::default() };
        f.render_widget(Paragraph::new(auth_text).block(Block::default().borders(Borders::ALL)).style(auth_style), form_layout[5]);

        draw_input(f, &app.password_key_input, form_layout[6], matches!(current_focus, Some(FormFocus::PasswordOrKey)));

        let submit_style = if matches!(current_focus, Some(FormFocus::Submit)) { Style::default().bg(Color::Green).fg(Color::Black) } else { Style::default() };
        f.render_widget(Paragraph::new("Submit").alignment(ratatui::layout::Alignment::Center).block(Block::default().borders(Borders::ALL)).style(submit_style), form_layout[7]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
