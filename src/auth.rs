use agol::models::ArcGISAccessToken;
use reqwest::header::{HeaderMap, HeaderValue, REFERER};
use serde::Deserialize;
use zeroize::Zeroize;

pub const CLIENT_ID_ENV: &str = "ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_ID";
pub const CLIENT_SECRET_ENV: &str = "ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_SECRET";

const ARCGIS_ROOT: &str = "https://www.arcgis.com";
const ARCGIS_REFERER: &str = "https://www.arcgis.com";

pub enum Credentials {
    Application {
        client_id: String,
        client_secret: String,
    },
    User {
        username: String,
        password: String,
    },
}

#[derive(Debug)]
pub struct AuthenticatedUser {
    pub token: ArcGISAccessToken,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct UserTokenResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
struct CurrentUser {
    username: String,
    #[serde(rename = "orgId")]
    org_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: ArcGisError,
}

#[derive(Debug, Deserialize)]
struct ArcGisError {
    code: Option<i64>,
    message: Option<String>,
    #[serde(default)]
    details: Vec<String>,
}

pub fn application_credentials() -> Option<Credentials> {
    let client_id = nonempty_env(CLIENT_ID_ENV)?;
    let client_secret = nonempty_env(CLIENT_SECRET_ENV)?;
    Some(Credentials::Application {
        client_id,
        client_secret,
    })
}

pub fn http_client() -> Result<reqwest::Client, String> {
    let mut headers = HeaderMap::new();
    headers.insert(REFERER, HeaderValue::from_static(ARCGIS_REFERER));
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|error| format!("Could not create the HTTP client: {error}"))
}

pub async fn authenticate(
    client: &reqwest::Client,
    credentials: Credentials,
) -> Result<AuthenticatedUser, String> {
    match credentials {
        Credentials::Application {
            client_id,
            mut client_secret,
        } => {
            let response = post_form(
                client,
                &format!("{ARCGIS_ROOT}/sharing/rest/oauth2/token"),
                &[
                    ("client_id", client_id.as_str()),
                    ("client_secret", client_secret.as_str()),
                    ("grant_type", "client_credentials"),
                    ("f", "json"),
                ],
            )
            .await;
            client_secret.zeroize();
            let body = response?;
            let response: OAuthTokenResponse = decode_response(&body)?;
            if response.access_token.is_empty() {
                return Err("ArcGIS Online returned an empty access token".to_string());
            }
            Ok(AuthenticatedUser {
                token: ArcGISAccessToken {
                    access_token: response.access_token,
                },
                username: None,
            })
        }
        Credentials::User {
            username,
            mut password,
        } => {
            let response = post_form(
                client,
                &format!("{ARCGIS_ROOT}/sharing/rest/generateToken"),
                &[
                    ("f", "json"),
                    ("username", username.as_str()),
                    ("password", password.as_str()),
                    ("client", "referer"),
                    ("referer", ARCGIS_REFERER),
                    ("expiration", "60"),
                ],
            )
            .await;
            password.zeroize();
            let body = response?;
            let response: UserTokenResponse = decode_response(&body)?;
            if response.token.is_empty() {
                return Err("ArcGIS Online returned an empty user token".to_string());
            }

            let token = ArcGISAccessToken {
                access_token: response.token,
            };
            let current_user = verify_user(client, &token).await?;
            if current_user
                .org_id
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                return Err("The ArcGIS account is not a member of an organization".to_string());
            }
            Ok(AuthenticatedUser {
                token,
                username: Some(current_user.username),
            })
        }
    }
}

async fn verify_user(
    client: &reqwest::Client,
    token: &ArcGISAccessToken,
) -> Result<CurrentUser, String> {
    let response = client
        .get(format!("{ARCGIS_ROOT}/sharing/rest/community/self"))
        .query(&[("f", "json"), ("token", token.access_token.as_str())])
        .send()
        .await
        .map_err(|error| format!("Could not verify the ArcGIS account: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read the ArcGIS verification response: {error}"))?;
    if !status.is_success() {
        return Err(format!(
            "ArcGIS account verification failed (HTTP {status})"
        ));
    }
    decode_response(&body)
}

async fn post_form(
    client: &reqwest::Client,
    url: &str,
    form: &[(&str, &str)],
) -> Result<String, String> {
    let response = client
        .post(url)
        .form(form)
        .send()
        .await
        .map_err(|error| format!("Could not connect to ArcGIS Online: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read the ArcGIS Online response: {error}"))?;
    if !status.is_success() {
        return Err(format!("ArcGIS Online returned HTTP {status}"));
    }
    Ok(body)
}

fn decode_response<T: for<'de> Deserialize<'de>>(body: &str) -> Result<T, String> {
    if let Ok(error) = serde_json::from_str::<ErrorEnvelope>(body) {
        return Err(format_arcgis_error(&error.error));
    }
    serde_json::from_str(body)
        .map_err(|_| "ArcGIS Online returned an unexpected authentication response".to_string())
}

fn format_arcgis_error(error: &ArcGisError) -> String {
    let code = error
        .code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let message = error.message.as_deref().unwrap_or("Authentication failed");
    if error.details.is_empty() {
        format!("ArcGIS Online error {code}: {message}")
    } else {
        format!(
            "ArcGIS Online error {code}: {message} ({})",
            error.details.join("; ")
        )
    }
}

fn nonempty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && !value.starts_with("__AGOLTUI_"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_arcgis_errors_without_exposing_response_fields() {
        let result = decode_response::<UserTokenResponse>(
            r#"{"error":{"code":400,"message":"Invalid username or password.","details":[]}}"#,
        );
        assert_eq!(
            result.unwrap_err(),
            "ArcGIS Online error 400: Invalid username or password."
        );
    }

    #[test]
    fn decodes_user_tokens() {
        let response = decode_response::<UserTokenResponse>(
            r#"{"token":"user-token","expires":123,"ssl":true}"#,
        )
        .unwrap();
        assert_eq!(response.token, "user-token");
    }
}
