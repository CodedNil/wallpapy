use crate::common::LoginPacket;
use anyhow::{Result, anyhow};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{body::Bytes, http::StatusCode, response::IntoResponse};
use bincode::{config::Configuration, serde::decode_from_slice};
use chrono::{DateTime, Utc};
use log::error;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncReadExt,
};
use uuid::Uuid;

const MIN_PASSWORD_LENGTH: usize = 6;
const TOKEN_LENGTH: usize = 20;
const AUTH_FILE: &str = "data/auth.ron";

#[derive(Serialize, Deserialize)]
struct Account {
    admin: bool,
    uuid: Uuid,
    username: String,
    password_hash: String,
    tokens: Vec<Token>,
}

#[derive(Serialize, Deserialize)]
struct Token {
    token: String,
    last_used: DateTime<Utc>,
}

type Accounts = HashMap<Uuid, Account>;

pub async fn login_server(packet: Bytes) -> impl IntoResponse {
    let (packet, _) =
        match decode_from_slice::<LoginPacket, Configuration>(&packet, bincode::config::standard())
        {
            Ok(value) => value,
            Err(e) => {
                error!("Failed to deserialise login packet: {e:?}");
                return (StatusCode::BAD_REQUEST, String::new());
            }
        };

    match login_impl(&packet).await {
        Ok(token) => (StatusCode::OK, token),
        Err(e) => {
            error!("Failed to login: {e:?}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

async fn read_accounts() -> Result<Accounts> {
    if fs::metadata(AUTH_FILE).await.is_err() {
        return Ok(HashMap::new());
    }

    let mut file = OpenOptions::new().read(true).open(AUTH_FILE).await?;
    let mut data = String::new();
    file.read_to_string(&mut data).await?;
    let accounts: Accounts = ron::from_str(&data)?;
    Ok(accounts)
}

async fn write_accounts(accounts: &Accounts) -> Result<()> {
    let pretty = ron::ser::PrettyConfig::new().compact_arrays(true);
    let data = ron::ser::to_string_pretty(accounts, pretty)?;
    fs::write(AUTH_FILE, data).await?;
    Ok(())
}

/// Login to account, returning a token
/// If no password is set, it will set the password
/// If no accounts exist, it will create an admin account
async fn login_impl(packet: &LoginPacket) -> Result<String> {
    let mut accounts = read_accounts().await.unwrap_or_default();

    // Create initial admin account if no accounts exist
    if accounts.is_empty() {
        if packet.password.len() < MIN_PASSWORD_LENGTH {
            return Err(anyhow!(
                "Password must be at least {MIN_PASSWORD_LENGTH} characters long"
            ));
        }

        // Hash the password
        let password_hash = Argon2::default()
            .hash_password(
                packet.password.as_bytes(),
                &SaltString::generate(&mut OsRng),
            )
            .map_err(|_| anyhow!("Failed to hash password"))?
            .to_string();

        // Create a new admin account
        let (token_entry, token) = generate_token();
        let new_account = Account {
            admin: true,
            uuid: Uuid::new_v4(),
            username: packet.username.clone(),
            password_hash,
            tokens: vec![token_entry],
        };

        // Serialize and save the admin account to the database
        accounts.insert(new_account.uuid, new_account);
        write_accounts(&accounts).await?;

        return Ok(format!("Admin Account Created|{token}"));
    }

    let Some(account) = accounts
        .values_mut()
        .find(|acc| acc.username == packet.username)
    else {
        return Err(anyhow!("Incorrect username or password"));
    };

    if account.password_hash.is_empty() {
        if packet.password.len() < MIN_PASSWORD_LENGTH {
            return Err(anyhow!(
                "Password must be at least {MIN_PASSWORD_LENGTH} characters long"
            ));
        }

        let password_hash = Argon2::default()
            .hash_password(
                packet.password.as_bytes(),
                &SaltString::generate(&mut OsRng),
            )
            .map_err(|_| anyhow!("Failed to hash password"))?
            .to_string();

        let (token_entry, token) = generate_token();
        account.tokens.push(token_entry);
        account.password_hash = password_hash;
        write_accounts(&accounts).await?;

        return Ok(format!("Admin Set|{token}"));
    }

    let parsed_hash = PasswordHash::new(&account.password_hash)
        .map_err(|_| anyhow!("Incorrect username or password"))?;

    if Argon2::default()
        .verify_password(packet.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(anyhow!("Incorrect username or password"));
    }

    let (token_entry, token) = generate_token();
    account.tokens.push(token_entry);
    write_accounts(&accounts).await?;
    Ok(token)
}

/// Helper function to generate a random token
fn generate_token() -> (Token, String) {
    let new_token: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(TOKEN_LENGTH)
        .map(char::from)
        .collect();
    let token = Token {
        token: new_token.clone(),
        last_used: Utc::now(),
    };
    (token, new_token)
}

/// Verify tokens, updating the `last_used`
pub async fn verify_token(input_token: &str) -> Result<bool> {
    let mut accounts = read_accounts().await?;

    for account in accounts.values_mut() {
        if let Some(token_entry) = account
            .tokens
            .iter_mut()
            .find(|token| token.token == input_token)
        {
            token_entry.last_used = Utc::now();
            write_accounts(&accounts).await?;
            return Ok(true);
        }
    }

    Ok(false)
}
