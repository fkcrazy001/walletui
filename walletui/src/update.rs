use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, EditMode, Field, Mode};

pub fn update(app: &mut App, key_event: KeyEvent) {
    if key_event.modifiers == KeyModifiers::CONTROL
        && matches!(key_event.code, KeyCode::Char('c') | KeyCode::Char('C'))
    {
        app.quit();
        return;
    }

    match app.mode.clone() {
        Mode::Normal => update_normal(app, key_event),
        Mode::Search => {
            if update_text(&mut app.query, key_event) {
                app.mode = Mode::Normal;
                app.selected = 0;
            }
        }
        Mode::Category => {
            if update_text(&mut app.category_filter, key_event) {
                app.mode = Mode::Normal;
                app.selected = 0;
            }
        }
        Mode::Editing { mode, field } => update_editing(app, key_event, mode, field),
        Mode::ConfirmDelete => update_confirm_delete(app, key_event),
    }
}

fn update_normal(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc | KeyCode::Char('q') => app.quit(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Char('a') => app.start_add(),
        KeyCode::Char('e') => app.start_edit(),
        KeyCode::Char('d') => app.start_delete(),
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.status = "输入搜索关键字".to_string();
        }
        KeyCode::Char('c') => {
            app.mode = Mode::Category;
            app.status = "输入分类过滤；清空表示全部分类".to_string();
        }
        KeyCode::Char('v') => app.show_password = !app.show_password,
        _ => {}
    }
}

fn update_text(value: &mut String, key_event: KeyEvent) -> bool {
    match key_event.code {
        KeyCode::Esc | KeyCode::Enter => return true,
        KeyCode::Backspace => {
            value.pop();
        }
        KeyCode::Char(ch) => value.push(ch),
        _ => {}
    }
    false
}

fn update_editing(app: &mut App, key_event: KeyEvent, edit_mode: EditMode, field: usize) {
    match key_event.code {
        KeyCode::Esc => app.cancel_mode(),
        KeyCode::Enter => app.save_form(edit_mode),
        KeyCode::Tab | KeyCode::Down => {
            app.mode = Mode::Editing {
                mode: edit_mode,
                field: (field + 1) % Field::ALL.len(),
            };
            app.move_form_cursor_to_end();
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.mode = Mode::Editing {
                mode: edit_mode,
                field: if field == 0 {
                    Field::ALL.len() - 1
                } else {
                    field - 1
                },
            };
            app.move_form_cursor_to_end();
        }
        KeyCode::Left => app.move_form_cursor_left(),
        KeyCode::Right => app.move_form_cursor_right(),
        KeyCode::Home => app.form_cursor = 0,
        KeyCode::End => app.move_form_cursor_to_end(),
        KeyCode::Backspace => app.backspace_form_char(),
        KeyCode::Char(ch) => {
            app.insert_form_char(ch);
        }
        _ => app.clamp_form_cursor(),
    }
}

fn update_confirm_delete(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete(),
        _ => app.cancel_mode(),
    }
}
