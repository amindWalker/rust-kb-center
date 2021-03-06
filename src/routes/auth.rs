#![warn(clippy::all)]

use crate::{
    db::Database,
    types::account::{Account, AccountId, Session},
};
use argon2::{self, Config};
use rand::{thread_rng, Rng};
use warp::Filter;

pub fn auth() -> impl Filter<Extract = (Session,), Error = warp::Rejection> + Clone {
    warp::header::<String>("Authorization").and_then(|token| {
        let token = match validate_token(token) {
            Ok(tk) => tk,
            Err(_) => return std::future::ready(Err(warp::reject::reject())),
        };

        std::future::ready(Ok(token))
    })
}

pub async fn register(
    account_db: Database,
    account: Account,
) -> Result<impl warp::Reply, warp::Rejection> {
    let password_hash = hash_password(account.password.as_bytes());

    let account = Account {
        id: account.id,
        email: account.email,
        password: password_hash,
    };

    match account_db.add_account(account).await {
        Ok(_) => Ok(warp::reply::with_status(
            "Account created successfully",
            warp::http::StatusCode::OK,
        )),
        Err(e) => Err(warp::reject::custom(e)),
    }
}

pub fn hash_password(password: &[u8]) -> String {
    let salt = thread_rng().gen::<[u8; 32]>();
    let config = Config::default();
    argon2::hash_encoded(password, &salt, &config).unwrap()
}

pub async fn login(
    account_db: Database,
    login: Account,
) -> Result<impl warp::Reply, warp::Rejection> {
    match account_db.get_account(login.email).await {
        Ok(acc) => match verify_password(&acc.password, login.password.as_bytes()) {
            Ok(valid) => {
                if valid {
                    Ok(warp::reply::json(&gen_token(acc.id.expect("ID not found"))))
                } else {
                    Err(warp::reject::custom(handle_errors::Error::WrongPassword))
                }
            }
            Err(e) => Err(warp::reject::custom(handle_errors::Error::LibArgonError(e))),
        },
        Err(e) => Err(warp::reject::custom(e)),
    }
}

pub fn verify_password(hash: &str, password: &[u8]) -> Result<bool, argon2::Error> {
    argon2::verify_encoded(hash, password)
}

fn gen_token(account_id: AccountId) -> String {
    let secret = std::env::var("PASETO_SECRET").unwrap();
    let date_time_now = chrono::Utc::now();
    let expiry_date = date_time_now + chrono::Duration::days(1);

    paseto::tokens::PasetoBuilder::new()
        .set_encryption_key(&Vec::from(secret.as_bytes()))
        .set_expiration(&expiry_date)
        .set_not_before(&chrono::Utc::now())
        .set_claim("account_id", serde_json::json!(account_id))
        .build()
        .expect("Unable to generate paseto token")
}

pub fn validate_token(token: String) -> Result<Session, handle_errors::Error> {
    let secret = std::env::var("PASETO_SECRET").unwrap();
    let token = paseto::tokens::validate_local_token(
        &token,
        None,
        secret.as_bytes(),
        &paseto::tokens::TimeBackend::Chrono,
    )
    .map_err(|_| handle_errors::Error::FailTokenDecryption)?;

    serde_json::from_value::<Session>(token).map_err(|_| handle_errors::Error::FailTokenDecryption)
}

#[cfg(test)]
mod auth_tests {
    use super::*;
    use color_eyre::Result;

    #[tokio::test]
    async fn post_test() -> Result<()> {
        color_eyre::install()?;

        std::env::set_var("PASETO_SECRET", "CHANGE THIS TO A SECRET DOV .ENV");
        let token = gen_token(AccountId(2));

        let access = auth();

        let response = warp::test::request()
            .header("Authorization", token)
            .filter(&access);
        assert_eq!(response.await.unwrap().account_id, AccountId(2));
        Ok(())
    }
}
