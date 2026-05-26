use std::path::PathBuf;

use color_eyre::Result;

use crate::storage::{self, PasswordEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Category,
    Title,
    Username,
    Password,
    Notes,
}

impl Field {
    pub const ALL: [Field; 5] = [
        Field::Category,
        Field::Title,
        Field::Username,
        Field::Password,
        Field::Notes,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Field::Category => "分类",
            Field::Title => "名称",
            Field::Username => "账号",
            Field::Password => "密码",
            Field::Notes => "备注",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    Add,
    Edit(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    Category,
    Editing { mode: EditMode, field: usize },
    ConfirmDelete,
}

#[derive(Debug)]
pub struct App {
    pub entries: Vec<PasswordEntry>,
    pub selected: usize,
    pub query: String,
    pub category_filter: String,
    pub mode: Mode,
    pub form: PasswordEntry,
    pub form_cursor: usize,
    pub show_password: bool,
    pub status: String,
    pub animation_tick: u64,
    pub should_quit: bool,
    master_key: String,
    vault_path: PathBuf,
}

impl App {
    pub fn new(master_key: String) -> Result<Self> {
        let vault_path = storage::default_vault_path();
        let entries = storage::load(&vault_path, &master_key)?;
        Ok(Self {
            entries,
            selected: 0,
            query: String::new(),
            category_filter: String::new(),
            mode: Mode::Normal,
            form: PasswordEntry::default(),
            form_cursor: 0,
            show_password: false,
            status: format!("Vault: {}", vault_path.display()),
            animation_tick: 0,
            should_quit: false,
            master_key,
            vault_path,
        })
    }

    pub fn tick(&mut self) {
        self.animation_tick = self.animation_tick.wrapping_add(1);
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();
        let category = self.category_filter.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let category_matches =
                    category.is_empty() || entry.category.to_lowercase().contains(&category);
                let query_matches = query.is_empty()
                    || entry.category.to_lowercase().contains(&query)
                    || entry.title.to_lowercase().contains(&query)
                    || entry.username.to_lowercase().contains(&query)
                    || entry.notes.to_lowercase().contains(&query);
                (category_matches && query_matches).then_some(index)
            })
            .collect()
    }

    pub fn selected_entry_index(&self) -> Option<usize> {
        self.filtered_indices().get(self.selected).copied()
    }

    pub fn selected_entry(&self) -> Option<&PasswordEntry> {
        self.selected_entry_index()
            .and_then(|index| self.entries.get(index))
    }

    pub fn move_down(&mut self) {
        let len = self.filtered_indices().len();
        if len > 0 {
            self.selected = (self.selected + 1).min(len - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn start_add(&mut self) {
        self.form = PasswordEntry::default();
        self.form_cursor = 0;
        self.mode = Mode::Editing {
            mode: EditMode::Add,
            field: 0,
        };
        self.status = "新增密码：Tab 切换字段，Enter 保存，Esc 取消".to_string();
    }

    pub fn start_edit(&mut self) {
        if let Some(index) = self.selected_entry_index() {
            self.form = self.entries[index].clone();
            self.form_cursor = self.form.category.chars().count();
            self.mode = Mode::Editing {
                mode: EditMode::Edit(index),
                field: 0,
            };
            self.status = "编辑密码：Tab 切换字段，Enter 保存，Esc 取消".to_string();
        }
    }

    pub fn start_delete(&mut self) {
        if self.selected_entry_index().is_some() {
            self.mode = Mode::ConfirmDelete;
            self.status = "确认删除？按 y 删除，其他键取消".to_string();
        }
    }

    pub fn confirm_delete(&mut self) {
        if let Some(index) = self.selected_entry_index() {
            let title = self.entries[index].title.clone();
            self.entries.remove(index);
            self.selected = self.selected.saturating_sub(1);
            self.finish_save(format!("已删除：{title}"));
        }
        self.mode = Mode::Normal;
    }

    pub fn cancel_mode(&mut self) {
        self.mode = Mode::Normal;
        self.status.clear();
    }

    pub fn save_form(&mut self, edit_mode: EditMode) {
        if self.form.title.trim().is_empty() {
            self.status = "名称不能为空".to_string();
            return;
        }
        if self.form.category.trim().is_empty() {
            self.form.category = "默认".to_string();
        }
        match edit_mode {
            EditMode::Add => {
                self.entries.push(self.form.trimmed());
                self.selected = self.filtered_indices().len().saturating_sub(1);
                self.finish_save("已新增密码".to_string());
            }
            EditMode::Edit(index) => {
                if let Some(entry) = self.entries.get_mut(index) {
                    *entry = self.form.trimmed();
                    self.finish_save("已更新密码".to_string());
                }
            }
        }
        self.mode = Mode::Normal;
    }

    pub fn finish_save(&mut self, success_message: String) {
        match storage::save(&self.vault_path, &self.master_key, &self.entries) {
            Ok(()) => self.status = success_message,
            Err(err) => self.status = format!("保存失败：{err}"),
        }
    }

    pub fn active_field(&self) -> Option<Field> {
        match self.mode {
            Mode::Editing { field, .. } => Field::ALL.get(field).copied(),
            _ => None,
        }
    }

    pub fn edit_field_mut(&mut self, field: Field) -> &mut String {
        match field {
            Field::Category => &mut self.form.category,
            Field::Title => &mut self.form.title,
            Field::Username => &mut self.form.username,
            Field::Password => &mut self.form.password,
            Field::Notes => &mut self.form.notes,
        }
    }

    pub fn active_field_len(&self) -> usize {
        self.active_field()
            .map(|field| self.edit_field(field).chars().count())
            .unwrap_or_default()
    }

    pub fn clamp_form_cursor(&mut self) {
        self.form_cursor = self.form_cursor.min(self.active_field_len());
    }

    pub fn move_form_cursor_left(&mut self) {
        self.form_cursor = self.form_cursor.saturating_sub(1);
    }

    pub fn move_form_cursor_right(&mut self) {
        self.form_cursor = (self.form_cursor + 1).min(self.active_field_len());
    }

    pub fn move_form_cursor_to_end(&mut self) {
        self.form_cursor = self.active_field_len();
    }

    pub fn insert_form_char(&mut self, ch: char) {
        if let Some(field) = self.active_field() {
            let cursor = self.form_cursor.min(self.edit_field(field).chars().count());
            let byte_index = char_to_byte_index(self.edit_field(field), cursor);
            self.edit_field_mut(field).insert(byte_index, ch);
            self.form_cursor = cursor + 1;
        }
    }

    pub fn backspace_form_char(&mut self) {
        if self.form_cursor == 0 {
            return;
        }
        if let Some(field) = self.active_field() {
            let cursor = self.form_cursor.min(self.edit_field(field).chars().count());
            let start = char_to_byte_index(self.edit_field(field), cursor - 1);
            let end = char_to_byte_index(self.edit_field(field), cursor);
            self.edit_field_mut(field).replace_range(start..end, "");
            self.form_cursor = cursor - 1;
        }
    }

    fn edit_field(&self, field: Field) -> &str {
        match field {
            Field::Category => &self.form.category,
            Field::Title => &self.form.title,
            Field::Username => &self.form.username,
            Field::Password => &self.form.password,
            Field::Notes => &self.form.notes,
        }
    }
}

fn char_to_byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_filter_matches_title_category_username_and_notes() {
        let mut app = App {
            entries: vec![
                PasswordEntry {
                    category: "work".into(),
                    title: "GitHub".into(),
                    username: "alice".into(),
                    password: "secret".into(),
                    notes: "recovery codes".into(),
                },
                PasswordEntry {
                    category: "bank".into(),
                    title: "Card".into(),
                    username: "bob".into(),
                    password: "pin".into(),
                    notes: String::new(),
                },
            ],
            selected: 0,
            query: "git".into(),
            category_filter: String::new(),
            mode: Mode::Normal,
            form: PasswordEntry::default(),
            form_cursor: 0,
            show_password: false,
            status: String::new(),
            animation_tick: 0,
            should_quit: false,
            master_key: "test".into(),
            vault_path: PathBuf::new(),
        };
        assert_eq!(app.filtered_indices(), vec![0]);

        app.query = "bob".into();
        assert_eq!(app.filtered_indices(), vec![1]);

        app.query.clear();
        app.category_filter = "wor".into();
        assert_eq!(app.filtered_indices(), vec![0]);
    }

    #[test]
    fn form_cursor_inserts_and_deletes_in_middle() {
        let mut app = App {
            entries: Vec::new(),
            selected: 0,
            query: String::new(),
            category_filter: String::new(),
            mode: Mode::Editing {
                mode: EditMode::Add,
                field: 1,
            },
            form: PasswordEntry {
                title: "ab".into(),
                ..PasswordEntry::default()
            },
            form_cursor: 1,
            show_password: false,
            status: String::new(),
            animation_tick: 0,
            should_quit: false,
            master_key: "test".into(),
            vault_path: PathBuf::new(),
        };

        app.insert_form_char('中');
        assert_eq!(app.form.title, "a中b");
        assert_eq!(app.form_cursor, 2);

        app.backspace_form_char();
        assert_eq!(app.form.title, "ab");
        assert_eq!(app.form_cursor, 1);
    }
}
