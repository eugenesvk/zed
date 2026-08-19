use anyhow::Result;
use collections::BTreeMap;
use credentials_provider::CredentialsProvider;
use futures::{FutureExt, StreamExt, future::BoxFuture};
use gpui::{App, AppContext, AsyncApp, Context, Entity, SharedString, Task};
use http_client::{CustomHeaders, HttpClient};
use language_model::{
    ApiKeyConfiguration, ApiKeyState, AuthenticateError, CompactionResult, EnvVar,
    FastModeConfirmation, IconOrSvg, LanguageModel, LanguageModelCompletionError,
    LanguageModelCompletionEvent, LanguageModelEffortLevel, LanguageModelId, LanguageModelName,
    LanguageModelProvider, LanguageModelProviderId, LanguageModelProviderName,
    LanguageModelProviderState, LanguageModelRequest, LanguageModelToolChoice, OPEN_AI_PROVIDER_ID,
    OPEN_AI_PROVIDER_NAME, ProviderSettingsView, RateLimiter, env_var,
};
use open_ai::{
    ResponseStreamEvent,
    responses::{
        CompactRequest, CompactedResponse, Request as ResponseRequest,
        StreamEvent as ResponsesStreamEvent, compact_response, stream_response,
    },
    stream_completion,
};
use settings::{OpenAiAvailableModel as AvailableModel, Settings, SettingsStore};
use std::sync::{Arc, LazyLock};
use strum::IntoEnumIterator;
use ui::IconName;

use open_ai::completion::token_usage_from_response_usage;
pub use open_ai::completion::{
    ChatCompletionMaxTokensParameter, OpenAiEventMapper, OpenAiResponseEventMapper, into_open_ai,
    into_open_ai_response,
};

const PROVIDER_ID: LanguageModelProviderId = OPEN_AI_PROVIDER_ID;
const PROVIDER_NAME: LanguageModelProviderName = OPEN_AI_PROVIDER_NAME;

const API_KEY_ENV_VAR_NAME: &str = "OPENAI_API_KEY";
static API_KEY_ENV_VAR: LazyLock<EnvVar> = env_var!(API_KEY_ENV_VAR_NAME);
































