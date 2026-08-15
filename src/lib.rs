use std::sync::Arc;

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::{Deserialize, Deserializer, Serialize};
use tokio::sync::RwLock;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Profile {
    pub display_name: String,
    pub nickname: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    profile: Arc<RwLock<Profile>>,
}

impl AppState {
    pub fn seeded() -> Self {
        Self {
            profile: Arc::new(RwLock::new(Profile {
                display_name: "田中太郎".to_owned(),
                nickname: Some("taro".to_owned()),
            })),
        }
    }

    pub async fn current_profile(&self) -> Profile {
        self.profile.read().await.clone()
    }
}

#[derive(Debug, Deserialize)]
struct UpdateProfileRequest {
    #[serde(default, deserialize_with = "deserialize_present_option")]
    nickname: Option<Option<String>>,
}

fn deserialize_present_option<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/profile", get(get_profile).patch(update_profile))
        .with_state(state)
}

async fn get_profile(State(state): State<AppState>) -> Json<Profile> {
    Json(state.current_profile().await)
}

async fn update_profile(
    State(state): State<AppState>,
    Json(request): Json<UpdateProfileRequest>,
) -> (StatusCode, Json<Profile>) {
    eprintln!("[api] deserialized nickname={:?}", request.nickname);

    let mut profile = state.profile.write().await;
    match request.nickname {
        Some(Some(nickname)) => profile.nickname = Some(nickname),
        Some(None) => profile.nickname = None,
        None => {}
    }

    eprintln!("[api] persisted nickname={:?}", profile.nickname);
    (StatusCode::OK, Json(profile.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, header::CONTENT_TYPE},
    };
    use tower::ServiceExt;

    async fn patch_profile(app: Router, body: &'static str) -> (StatusCode, Profile) {
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/profile")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .expect("テスト用リクエストを構築できませんでした"),
            )
            .await
            .expect("ルーターが応答を返しませんでした");

        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("レスポンスボディを読めませんでした");
        let profile = serde_json::from_slice(&body).expect("レスポンスJSONを読めませんでした");
        (status, profile)
    }

    #[tokio::test]
    async fn null_nickname_must_clear_the_persisted_value() {
        let state = AppState::seeded();
        let (status, response_profile) =
            patch_profile(app(state.clone()), r#"{"nickname":null}"#).await;
        let persisted_profile = state.current_profile().await;

        assert!(
            status == StatusCode::OK
                && response_profile.nickname.is_none()
                && persisted_profile.nickname.is_none(),
            "nickname:nullは200とnickname=nullを返し、保存済みのnicknameも消去する必要があります: status={status}, response={:?}, persisted={:?}",
            response_profile.nickname,
            persisted_profile.nickname,
        );
    }

    #[tokio::test]
    async fn omitted_nickname_must_preserve_the_persisted_value() {
        let state = AppState::seeded();
        let (status, response_profile) = patch_profile(app(state.clone()), "{}").await;
        let persisted_profile = state.current_profile().await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response_profile.nickname.as_deref(), Some("taro"));
        assert_eq!(persisted_profile.nickname.as_deref(), Some("taro"));
    }

    #[tokio::test]
    async fn string_nickname_must_replace_the_persisted_value() {
        let state = AppState::seeded();
        let (status, response_profile) =
            patch_profile(app(state.clone()), r#"{"nickname":"hanako"}"#).await;
        let persisted_profile = state.current_profile().await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response_profile.nickname.as_deref(), Some("hanako"));
        assert_eq!(persisted_profile.nickname.as_deref(), Some("hanako"));
    }
}
