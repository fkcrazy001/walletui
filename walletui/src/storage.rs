use std::{
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use color_eyre::{Result, eyre::eyre};

use crate::crypto;

const MAGIC: &[u8] = b"WALLETUI1";
const PBKDF2_ROUNDS: u32 = 120_000;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PasswordEntry {
    pub category: String,
    pub title: String,
    pub username: String,
    pub password: String,
    pub notes: String,
}

impl PasswordEntry {
    pub fn trimmed(&self) -> Self {
        Self {
            category: self.category.trim().to_string(),
            title: self.title.trim().to_string(),
            username: self.username.trim().to_string(),
            password: self.password.clone(),
            notes: self.notes.trim().to_string(),
        }
    }
}

pub fn default_vault_path() -> PathBuf {
    if let Ok(path) = env::var("WALLETUI_VAULT") {
        return PathBuf::from(path);
    }
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".walletui.vault")
}

pub fn load(path: &Path, master_key: &str) -> Result<Vec<PasswordEntry>> {
    match fs::read(path) {
        Ok(bytes) => decrypt_entries(&bytes, master_key),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err.into()),
    }
}

pub fn save(path: &Path, master_key: &str, entries: &[PasswordEntry]) -> Result<()> {
    let bytes = encrypt_entries(entries, master_key)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn encrypt_entries(entries: &[PasswordEntry], master_key: &str) -> Result<Vec<u8>> {
    let salt = random_bytes::<16>();
    let nonce = random_bytes::<16>();
    let key = crypto::pbkdf2_sha256(master_key.as_bytes(), &salt, PBKDF2_ROUNDS);
    let plaintext = encode_entries(entries);
    let ciphertext = crypto::xor_stream(&key, &nonce, &plaintext);
    let mac_data = mac_data(&salt, &nonce, &ciphertext);
    let mac = crypto::hmac_sha256(&key, &mac_data);

    let mut output = Vec::new();
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&PBKDF2_ROUNDS.to_le_bytes());
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);
    output.extend_from_slice(&mac);
    Ok(output)
}

fn decrypt_entries(bytes: &[u8], master_key: &str) -> Result<Vec<PasswordEntry>> {
    let header_len = MAGIC.len() + 4 + 16 + 16 + 32;
    if bytes.len() < header_len || &bytes[..MAGIC.len()] != MAGIC {
        return Err(eyre!("vault 格式不正确"));
    }

    let mut cursor = MAGIC.len();
    let mut rounds = [0u8; 4];
    rounds.copy_from_slice(&bytes[cursor..cursor + 4]);
    cursor += 4;
    let rounds = u32::from_le_bytes(rounds);
    let salt = &bytes[cursor..cursor + 16];
    cursor += 16;
    let nonce = &bytes[cursor..cursor + 16];
    cursor += 16;
    let mac_start = bytes.len() - 32;
    let ciphertext = &bytes[cursor..mac_start];
    let expected_mac = &bytes[mac_start..];

    let key = crypto::pbkdf2_sha256(master_key.as_bytes(), salt, rounds);
    let mac_data = mac_data(salt, nonce, ciphertext);
    let actual_mac = crypto::hmac_sha256(&key, &mac_data);
    if !crypto::constant_time_eq(expected_mac, &actual_mac) {
        return Err(eyre!("主密码错误或 vault 已损坏"));
    }

    let plaintext = crypto::xor_stream(&key, nonce, ciphertext);
    decode_entries(&plaintext)
}

fn mac_data(salt: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(salt.len() + nonce.len() + ciphertext.len());
    data.extend_from_slice(salt);
    data.extend_from_slice(nonce);
    data.extend_from_slice(ciphertext);
    data
}

fn encode_entries(entries: &[PasswordEntry]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for entry in entries {
        write_string(&mut bytes, &entry.category);
        write_string(&mut bytes, &entry.title);
        write_string(&mut bytes, &entry.username);
        write_string(&mut bytes, &entry.password);
        write_string(&mut bytes, &entry.notes);
    }
    bytes
}

fn decode_entries(bytes: &[u8]) -> Result<Vec<PasswordEntry>> {
    let mut cursor = 0;
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push(PasswordEntry {
            category: read_string(bytes, &mut cursor)?,
            title: read_string(bytes, &mut cursor)?,
            username: read_string(bytes, &mut cursor)?,
            password: read_string(bytes, &mut cursor)?,
            notes: read_string(bytes, &mut cursor)?,
        });
    }
    Ok(entries)
}

fn write_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32> {
    if bytes.len().saturating_sub(*cursor) < 4 {
        return Err(eyre!("vault 数据截断"));
    }
    let mut value = [0u8; 4];
    value.copy_from_slice(&bytes[*cursor..*cursor + 4]);
    *cursor += 4;
    Ok(u32::from_le_bytes(value))
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String> {
    let len = read_u32(bytes, cursor)? as usize;
    if bytes.len().saturating_sub(*cursor) < len {
        return Err(eyre!("vault 数据截断"));
    }
    let value = String::from_utf8(bytes[*cursor..*cursor + len].to_vec())?;
    *cursor += len;
    Ok(value)
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    if let Ok(mut random) = fs::File::open("/dev/urandom")
        && random.read_exact(&mut bytes).is_ok()
    {
        return bytes;
    }
    if bytes.iter().any(|byte| *byte != 0) {
        return bytes;
    }

    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    for byte in &mut bytes {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        *byte = (seed & 0xff) as u8;
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_round_trip_through_encrypted_vault() {
        let entries = vec![PasswordEntry {
            category: "work".into(),
            title: "GitHub".into(),
            username: "alice".into(),
            password: "secret".into(),
            notes: "2fa enabled".into(),
        }];

        let encrypted = encrypt_entries(&entries, "master").unwrap();
        assert!(!String::from_utf8_lossy(&encrypted).contains("secret"));
        assert_eq!(decrypt_entries(&encrypted, "master").unwrap(), entries);
        assert!(decrypt_entries(&encrypted, "wrong").is_err());
    }
}
