use std::path::Path;
use std::sync::Arc;

use codex_agent_identity::AgentIdentityKey;
use codex_agent_identity::AgentTaskAuthorizationTarget;
use codex_agent_identity::authorization_header_for_agent_task;
use codex_api::AuthProvider;
use codex_api::SharedAuthProvider;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_login::KIMI_CODE_PROVIDER_ID;
use codex_login::kimi_code;
use codex_login::kimi_code::KimiCodeAuthError;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::bundled_provider_catalog_entry_for_base_url;
use codex_protocol::error::CodexErr;
use http::HeaderMap;
use http::HeaderValue;

use crate::bearer_auth_provider::BearerAuthProvider;

const BEDROCK_API_KEY_UNSUPPORTED_MESSAGE: &str =
    "Bedrock API key auth is only supported by the Amazon Bedrock model provider";

#[derive(Clone, Debug)]
struct AgentIdentityAuthProvider {
    auth: codex_login::auth::AgentIdentityAuth,
}

impl AuthProvider for AgentIdentityAuthProvider {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        let record = self.auth.record();
        let header_value = authorization_header_for_agent_task(
            AgentIdentityKey {
                agent_runtime_id: &record.agent_runtime_id,
                private_key_pkcs8_base64: &record.agent_private_key,
            },
            AgentTaskAuthorizationTarget {
                agent_runtime_id: &record.agent_runtime_id,
                task_id: self.auth.process_task_id(),
            },
        )
        .map_err(std::io::Error::other);

        if let Ok(header_value) = header_value
            && let Ok(header) = HeaderValue::from_str(&header_value)
        {
            let _ = headers.insert(http::header::AUTHORIZATION, header);
        }

        if let Ok(header) = HeaderValue::from_str(self.auth.account_id()) {
            let _ = headers.insert("ChatGPT-Account-ID", header);
        }

        if self.auth.is_fedramp_account() {
            let _ = headers.insert("X-OpenAI-Fedramp", HeaderValue::from_static("true"));
        }
    }
}

// Some providers are meant to send no auth headers. Examples include local OSS
// providers and custom test providers with `requires_openai_auth = false`.
#[derive(Clone, Debug)]
struct UnauthenticatedAuthProvider;

impl AuthProvider for UnauthenticatedAuthProvider {
    fn add_auth_headers(&self, _headers: &mut HeaderMap) {}
}

pub fn unauthenticated_auth_provider() -> SharedAuthProvider {
    Arc::new(UnauthenticatedAuthProvider)
}

/// Returns the provider-scoped auth manager when this provider uses command-backed auth.
///
/// Providers without custom auth continue using the caller-supplied base manager, when present.
pub(crate) fn auth_manager_for_provider(
    auth_manager: Option<Arc<AuthManager>>,
    provider: &ModelProviderInfo,
) -> Option<Arc<AuthManager>> {
    match provider.auth.clone() {
        Some(config) => Some(AuthManager::external_bearer_only(config)),
        None => auth_manager,
    }
}

pub(crate) async fn resolve_provider_auth(
    auth: Option<&CodexAuth>,
    provider: &ModelProviderInfo,
    codex_home: Option<&Path>,
) -> codex_protocol::error::Result<SharedAuthProvider> {
    if matches!(auth, Some(CodexAuth::BedrockApiKey(_))) {
        return Err(CodexErr::UnsupportedOperation(
            BEDROCK_API_KEY_UNSUPPORTED_MESSAGE.to_string(),
        ));
    }

    match bearer_auth_for_provider(provider) {
        Ok(Some(auth)) => return Ok(Arc::new(auth)),
        Ok(None) => {}
        Err(err) => {
            // The Kimi Code provider can authenticate with stored OAuth
            // credentials when no API key is available in the environment.
            return match kimi_code_bearer_token(provider, codex_home).await? {
                Some(token) => Ok(Arc::new(BearerAuthProvider::new(token))),
                None => Err(err),
            };
        }
    }

    Ok(match auth {
        Some(auth) => auth_provider_from_auth(auth),
        None => unauthenticated_auth_provider(),
    })
}

/// Resolves the stored Kimi Code OAuth access token for the `kimi-for-coding`
/// provider, refreshing it first when it is stale.
///
/// Returns `Ok(None)` when the provider is not Kimi Code, no Codex home is
/// available, or no credentials have been stored by the device login flow.
async fn kimi_code_bearer_token(
    provider: &ModelProviderInfo,
    codex_home: Option<&Path>,
) -> codex_protocol::error::Result<Option<String>> {
    if !is_kimi_code_provider(provider) {
        return Ok(None);
    }
    let Some(codex_home) = codex_home else {
        return Ok(None);
    };
    match kimi_code::resolve_access_token(codex_home).await {
        Ok(token) => Ok(Some(token)),
        Err(KimiCodeAuthError::MissingCredentials) => Ok(None),
        Err(err) => Err(CodexErr::Fatal(format!(
            "Kimi Code authentication failed: {err}"
        ))),
    }
}

fn is_kimi_code_provider(provider: &ModelProviderInfo) -> bool {
    provider.base_url.as_deref().is_some_and(|base_url| {
        bundled_provider_catalog_entry_for_base_url(base_url)
            .is_some_and(|entry| entry.id == KIMI_CODE_PROVIDER_ID)
    })
}

fn bearer_auth_for_provider(
    provider: &ModelProviderInfo,
) -> codex_protocol::error::Result<Option<BearerAuthProvider>> {
    if let Some(api_key) = provider.api_key()? {
        return Ok(Some(provider_auth_token(api_key, provider)));
    }

    if let Some(token) = provider.experimental_bearer_token.clone() {
        return Ok(Some(provider_auth_token(token, provider)));
    }

    Ok(None)
}

fn provider_auth_token(token: String, provider: &ModelProviderInfo) -> BearerAuthProvider {
    BearerAuthProvider {
        token: Some(token),
        account_id: None,
        is_fedramp_account: false,
        token_header_name: provider.is_anthropic_provider().then_some("x-api-key"),
        use_bearer_prefix: !provider.is_anthropic_provider(),
    }
}

/// Builds request-header auth for a first-party Codex auth snapshot.
pub fn auth_provider_from_auth(auth: &CodexAuth) -> SharedAuthProvider {
    match auth {
        CodexAuth::AgentIdentity(auth) => {
            Arc::new(AgentIdentityAuthProvider { auth: auth.clone() })
        }
        CodexAuth::BedrockApiKey(_) => unreachable!("{BEDROCK_API_KEY_UNSUPPORTED_MESSAGE}"),
        CodexAuth::ApiKey(_)
        | CodexAuth::Chatgpt(_)
        | CodexAuth::ChatgptAuthTokens(_)
        | CodexAuth::PersonalAccessToken(_) => Arc::new(BearerAuthProvider {
            token: auth.get_token().ok(),
            account_id: auth.get_account_id(),
            is_fedramp_account: auth.is_fedramp_account(),
            token_header_name: None,
            use_bearer_prefix: true,
        }),
    }
}

#[cfg(test)]
mod tests {
    use codex_login::auth::BedrockApiKeyAuth;
    use codex_model_provider_info::WireApi;
    use codex_model_provider_info::create_oss_provider_with_base_url;
    use pretty_assertions::assert_eq;

    use super::*;

    #[tokio::test]
    async fn unauthenticated_auth_provider_adds_no_headers() {
        let provider =
            create_oss_provider_with_base_url("http://localhost:11434/v1", WireApi::Responses);
        let auth = resolve_provider_auth(/*auth*/ None, &provider, /*codex_home*/ None)
            .await
            .expect("auth should resolve");

        assert!(auth.to_auth_headers().is_empty());
    }

    #[test]
    fn kimi_for_coding_provider_is_detected_by_base_url() {
        let provider = ModelProviderInfo {
            name: "Kimi For Coding".to_string(),
            base_url: Some("https://api.kimi.com/coding/v1".to_string()),
            env_key: Some("KIMI_API_KEY".to_string()),
            wire_api: WireApi::Chat,
            requires_openai_auth: false,
            ..Default::default()
        };

        assert!(is_kimi_code_provider(&provider));
        assert!(!is_kimi_code_provider(&create_oss_provider_with_base_url(
            "http://localhost:11434/v1",
            WireApi::Responses,
        )));
    }

    #[tokio::test]
    async fn openai_provider_rejects_bedrock_api_key_auth() {
        let provider = ModelProviderInfo::create_openai_provider(/*base_url*/ None);
        let auth = CodexAuth::BedrockApiKey(BedrockApiKeyAuth {
            api_key: "bedrock-api-key-test".to_string(),
            region: "us-east-1".to_string(),
        });

        match resolve_provider_auth(Some(&auth), &provider, /*codex_home*/ None).await {
            Err(CodexErr::UnsupportedOperation(message)) => {
                assert_eq!(message, BEDROCK_API_KEY_UNSUPPORTED_MESSAGE);
            }
            Err(err) => panic!("unexpected auth error: {err:?}"),
            Ok(_) => panic!("Bedrock API key auth should be rejected"),
        }
    }
}
