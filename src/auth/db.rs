use deadpool_postgres::Client;
use std::io;
use tokio_pg_mapper::FromTokioPostgresRow;

use crate::errors::ServiceError;

use super::model::*;

pub async fn add_user(client: &Client, usr: CreateUser) -> Result<CreatedUser, ServiceError> {
    let statement = client
        .prepare(  "INSERT INTO public.users (id, username, hashed_password, email) VALUES ($0, $1 ) RETURNING id",)
        .await
        .unwrap();

    let result = client
        .query_one(&statement, &[&usr.email, &usr.hashed_password])
        .await?;
    let user = CreatedUser::from_row_ref(&result).unwrap(); // or from_row_ref(&result)
    Ok(user)
}

pub async fn add_session(
    client: &Client,
    sess: SessionAdd,
) -> Result<CreatedSession, ServiceError> {
    let statement = client
        .prepare(
            "INSERT INTO public.sessions (user_id, session_verifier, otp_code_encrypted)
            VALUES($1, $2, $3) RETURNING id",
        )
        .await
        .unwrap();

    let result = client
        .query_one(
            &statement,
            &[&sess.user_id, &sess.session_verifier, &sess.otp_code_encr],
        )
        .await?;
    let sess = CreatedSession::from_row_ref(&result).unwrap(); // or from_row_ref(&result)
    Ok(sess)
}

pub async fn delete_session(client: &Client, sess: Session) -> Result<(), io::Error> {
    let statement = client
        .prepare("DELETE FROM public.sessions WHERE id = $1")
        .await
        .unwrap();

    client
        .execute(&statement, &[&sess.session_id])
        .await
        .unwrap();
    Ok(())
}

pub async fn find_user_by_session(client: &Client, session: Session) -> Option<UserSession> {
    let statement = client
        .prepare(
            " SELECT
            id,
            user_id,
            session_verifier,
            otp_code_confirmed,
            otp_code_encrypted,
            otp_code_attempts,
            otp_code_sent FROM public.users WHERE id = $1",
        )
        .await
        //  .map_err(|e| format!("Error preparing statement: {}", e))?;
        .unwrap();

    let maybe_session = client
        .query_opt(&statement, &[&session.session_verifier])
        .await
        // .ok()
        .expect("Error adding session ")
        .map(|row| UserSession::from_row_ref(&row).unwrap());
    // .map(|row| UserSession::from_row(row))
    // .filter(|ses| constant_time_compare(&sess.session_verifier, &sess.session_verifier));

    maybe_session.and_then(|sess| {
        if constant_time_compare(&session.session_verifier, &sess.session_verifier) {
            Some(sess)
        } else {
            None
        }
    })
}

// Constant time string compare.
pub fn constant_time_compare(a: &str, b: &str) -> bool {
    a.bytes()
        .zip(b.bytes())
        .fold(0, |acc, (a, b)| acc | (a ^ b))
        == 0
}

pub async fn find_user_by_mail(client: &Client, email: String) -> Result<FindUser, io::Error> {
    let statement = client
        .prepare("SELECT id, hashed_password FROM public.users WHERE email = $1")
        .await
        .unwrap();

    let maybe_user = client
        .query_opt(&statement, &[&email])
        .await
        .expect("Error adding category ")
        .map(|row| FindUser::from_row_ref(&row).unwrap());

    match maybe_user {
        Some(user) => Ok(user),
        None => Err(io::Error::new(io::ErrorKind::NotFound, "Not found")),
    }
}

pub async fn find_user_mail_by_id(client: &Client, id: i32) -> Result<UserMail, ServiceError> {
    let statement = client
        .prepare("SELECT email FROM public.users WHERE id = $1")
        .await
        .unwrap();

    let maybe_user = client
        .query_opt(&statement, &[&id])
        .await
        .expect("Error fetching mail ")
        .map(|row| UserMail::from_row_ref(&row).unwrap());

    match maybe_user {
        Some(user) => Ok(user),
        None => Err(ServiceError::BadId),
    }
}

pub async fn session_otp_update_true(client: &Client, id: i32) -> Result<(), io::Error> {
    let statement = client
        .prepare("update public.sessions set otp_code_sent = true where id = $1")
        .await
        .unwrap();

    let result = client
        .execute(&statement, &[&id])
        .await
        .expect("Error getting todo lists");

    match result {
        ref updated if *updated == 1 => Ok(()),
        _ => Err(io::Error::new(io::ErrorKind::Other, "Failed to check list")),
    }
}

pub async fn session_otp_update_confirm_true(client: &Client, id: i32) -> Result<(), io::Error> {
    let statement = client
        .prepare(
            "update public.sessions SET otp_code_confirm = true AND otp_code_attempts = 0 WHERE id = $0",
        )
        .await
        .unwrap();

    let result = client
        .execute(&statement, &[&id])
        .await
        .expect("Error getting todo lists");

    match result {
        ref updated if *updated == 1 => Ok(()),
        _ => Err(io::Error::new(io::ErrorKind::Other, "Failed to check list")),
    }
}

pub async fn session_otp_set_attempts(client: &Client, id: i32) -> Result<(), io::Error> {
    let statement = client
        .prepare(
            "update public.sessions SET otp_code_attempts = otp_code_attempts + 1 where id = $1",
        )
        .await
        .unwrap();

    let result = client
        .execute(&statement, &[&id])
        .await
        .expect("Error getting todo lists");

    match result {
        ref updated if *updated == 1 => Ok(()),
        _ => Err(io::Error::new(io::ErrorKind::Other, "Failed to check list")),
    }
}
