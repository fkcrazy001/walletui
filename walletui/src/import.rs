use std::{fs, path::Path};

use color_eyre::{Result, eyre::eyre};

use crate::storage::PasswordEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    Csv,
    Tsv,
}

impl ImportFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "csv" => Ok(Self::Csv),
            "tsv" => Ok(Self::Tsv),
            _ => Err(eyre!("不支持的导入格式：{value}，可选 csv 或 tsv")),
        }
    }

    pub fn delimiter(self) -> char {
        match self {
            Self::Csv => ',',
            Self::Tsv => '\t',
        }
    }
}

pub fn read_entries(path: &Path, format: ImportFormat) -> Result<Vec<PasswordEntry>> {
    let content = fs::read_to_string(path)?;
    parse_entries(&content, format)
}

fn parse_entries(content: &str, format: ImportFormat) -> Result<Vec<PasswordEntry>> {
    let delimiter = format.delimiter();
    let mut rows = Vec::new();
    for (line_index, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = parse_delimited_line(line, delimiter)
            .map_err(|err| eyre!("第 {} 行解析失败：{err}", line_index + 1))?;
        rows.push((line_index + 1, fields));
    }

    if rows
        .first()
        .is_some_and(|(_, fields)| looks_like_header(fields))
    {
        rows.remove(0);
    }

    rows.into_iter()
        .map(|(line, fields)| row_to_entry(line, fields))
        .collect()
}

fn row_to_entry(line: usize, fields: Vec<String>) -> Result<PasswordEntry> {
    if fields.len() < 4 || fields.len() > 5 {
        return Err(eyre!(
            "第 {line} 行字段数量不正确，需要 4 或 5 列：category,title,username,password,notes"
        ));
    }
    let mut entry = PasswordEntry {
        category: fields[0].trim().to_string(),
        title: fields[1].trim().to_string(),
        username: fields[2].trim().to_string(),
        password: fields[3].clone(),
        notes: fields
            .get(4)
            .map(|value| value.trim())
            .unwrap_or("")
            .to_string(),
    };
    if entry.category.is_empty() {
        entry.category = "默认".to_string();
    }
    if entry.title.is_empty() {
        return Err(eyre!("第 {line} 行名称不能为空"));
    }
    Ok(entry)
}

fn looks_like_header(fields: &[String]) -> bool {
    let normalized = fields
        .iter()
        .map(|field| field.trim().to_lowercase())
        .collect::<Vec<_>>();
    normalized.len() >= 4
        && matches!(normalized[0].as_str(), "category" | "分类")
        && matches!(normalized[1].as_str(), "title" | "name" | "名称")
        && matches!(normalized[2].as_str(), "username" | "account" | "账号")
        && matches!(normalized[3].as_str(), "password" | "密码")
}

fn parse_delimited_line(line: &str, delimiter: char) -> Result<Vec<String>> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' && current.is_empty() {
            in_quotes = true;
        } else if ch == delimiter {
            fields.push(current);
            current = String::new();
        } else {
            current.push(ch);
        }
    }

    if in_quotes {
        return Err(eyre!("引号未闭合"));
    }
    fields.push(current);
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_csv_with_header_and_quotes() {
        let input = "category,title,username,password,notes\nwork,GitHub,alice,\"s,ecret\",\"has \"\"2fa\"\"\"\n";
        let entries = parse_entries(input, ImportFormat::Csv).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "GitHub");
        assert_eq!(entries[0].password, "s,ecret");
        assert_eq!(entries[0].notes, "has \"2fa\"");
    }

    #[test]
    fn parses_tsv_without_notes() {
        let input = "bank\tCard\tbob\tpin\n";
        let entries = parse_entries(input, ImportFormat::Tsv).unwrap();
        assert_eq!(entries[0].category, "bank");
        assert_eq!(entries[0].notes, "");
    }
}
