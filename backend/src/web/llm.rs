// LLM Web API - REST endpoints for LLM configuration and chat

use axum::{
    body::Body,
    extract::State,
    http::header,
    response::Response,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;

use crate::llm::{
    config::{LlmConfig, LlmConfigRequest},
    types::{get_provider_presets, ChatCompletionChunk, ChatMessage, ProviderPreset, Role, StreamEvent},
    FunctionRegistry, LlmClient,
};
use crate::state::AppState;
use crate::web::auth::ApiError;

/// LLM API State
#[derive(Clone)]
pub struct LlmState {
    pub app_state: Arc<AppState>,
}

/// Create LLM router
pub fn llm_router() -> Router<LlmState> {
    Router::new()
        // Configuration endpoints
        .route("/config", get(get_configs))
        .route("/config", post(create_config))
        .route("/config/:id", put(update_config))
        .route("/config/:id", delete(delete_config))
        .route("/config/:id/enable", post(enable_config))
        .route("/config/test", post(test_connection))
        // Provider presets
        .route("/providers", get(get_providers))
        // Chat endpoints
        .route("/chat", post(chat))
        .route("/chat/stream", post(chat_stream))
        .route("/tools", get(get_tools))
        // Session management
        .route("/sessions", get(get_sessions))
        .route("/sessions", post(create_session))
        .route("/sessions/:id", get(get_session_messages))
        .route("/sessions/:id", delete(delete_session))
        .route("/sessions/:id/title", patch(update_session_title))
        // Legacy endpoints (for backward compatibility)
        .route("/conversations", get(get_conversations))
        .route("/conversations", delete(clear_conversations))
}

/// Helper to create internal error
fn internal_error(msg: impl ToString) -> ApiError {
    ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: msg.to_string(),
        details: None,
    }
}

/// Helper to create not found error
fn not_found(msg: impl ToString) -> ApiError {
    ApiError {
        code: "NOT_FOUND".to_string(),
        message: msg.to_string(),
        details: None,
    }
}

/// Helper to create bad request error
fn bad_request(msg: impl ToString) -> ApiError {
    ApiError {
        code: "BAD_REQUEST".to_string(),
        message: msg.to_string(),
        details: None,
    }
}

/// Get all LLM configurations
async fn get_configs(
    State(state): State<LlmState>,
) -> Result<Json<Vec<LlmConfigResponse>>, ApiError> {
    let configs = sqlx::query_as::<_, (i64, String, String, String, String, String, bool)>(
        "SELECT id, provider, display_name, api_base_url, api_key, model, enabled FROM llm_config"
    )
    .fetch_all(state.app_state.db.pool())
    .await
    .map_err(|e| internal_error(e))?;

    let response: Vec<LlmConfigResponse> = configs
        .into_iter()
        .map(|(id, provider, display_name, api_base_url, api_key, model, enabled)| {
            LlmConfigResponse {
                id,
                provider,
                display_name,
                api_base_url,
                api_key: api_key.clone(),
                api_key_masked: mask_api_key(&api_key),
                model,
                enabled,
            }
        })
        .collect();

    Ok(Json(response))
}

/// Create a new LLM configuration
async fn create_config(
    State(state): State<LlmState>,
    Json(req): Json<LlmConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let display_name = req.display_name.unwrap_or_else(|| req.provider.clone());
    
    sqlx::query(
        "INSERT INTO llm_config (provider, display_name, api_base_url, api_key, model, enabled) VALUES (?, ?, ?, ?, ?, 0)"
    )
    .bind(&req.provider)
    .bind(&display_name)
    .bind(&req.api_base_url)
    .bind(&req.api_key)
    .bind(&req.model)
    .execute(state.app_state.db.pool())
    .await
    .map_err(|e| internal_error(e))?;

    Ok(Json(serde_json::json!({"success": true, "message": "配置已创建"})))
}

/// Update an existing configuration
async fn update_config(
    State(state): State<LlmState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<LlmConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let display_name = req.display_name.unwrap_or_else(|| req.provider.clone());
    
    let result = if req.api_key.is_empty() {
        // If API key is empty, don't update it
        sqlx::query(
            "UPDATE llm_config SET provider = ?, display_name = ?, api_base_url = ?, model = ? WHERE id = ?"
        )
        .bind(&req.provider)
        .bind(&display_name)
        .bind(&req.api_base_url)
        .bind(&req.model)
        .bind(id)
        .execute(state.app_state.db.pool())
        .await
    } else {
        sqlx::query(
            "UPDATE llm_config SET provider = ?, display_name = ?, api_base_url = ?, api_key = ?, model = ? WHERE id = ?"
        )
        .bind(&req.provider)
        .bind(&display_name)
        .bind(&req.api_base_url)
        .bind(&req.api_key)
        .bind(&req.model)
        .bind(id)
        .execute(state.app_state.db.pool())
        .await
    };

    let result = result.map_err(|e| internal_error(e))?;

    if result.rows_affected() == 0 {
        return Err(not_found("配置不存在"));
    }

    Ok(Json(serde_json::json!({"success": true})))
}

/// Delete a configuration
async fn delete_config(
    State(state): State<LlmState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query("DELETE FROM llm_config WHERE id = ?")
        .bind(id)
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    if result.rows_affected() == 0 {
        return Err(not_found("配置不存在"));
    }

    Ok(Json(serde_json::json!({"success": true})))
}

/// Enable a specific configuration (disables others)
async fn enable_config(
    State(state): State<LlmState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Disable all configs first
    sqlx::query("UPDATE llm_config SET enabled = 0")
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    // Enable the selected one
    let result = sqlx::query("UPDATE llm_config SET enabled = 1 WHERE id = ?")
        .bind(id)
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    if result.rows_affected() == 0 {
        return Err(not_found("配置不存在"));
    }

    Ok(Json(serde_json::json!({"success": true, "message": "已启用"})))
}

/// Test LLM connection
async fn test_connection(
    State(state): State<LlmState>,
    Json(req): Json<LlmConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let config = LlmConfig {
        id: 0,
        provider: req.provider,
        display_name: req.display_name.unwrap_or_default(),
        api_base_url: req.api_base_url,
        api_key: req.api_key,
        model: req.model,
        enabled: true,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    let registry = Arc::new(FunctionRegistry::new(state.app_state.clone()));
    let client = LlmClient::new(config, registry);
    
    match client.test_connection().await {
        Ok(true) => Ok(Json(serde_json::json!({"success": true, "message": "连接成功"}))),
        Ok(false) => Ok(Json(serde_json::json!({"success": false, "message": "连接失败"}))),
        Err(e) => Ok(Json(serde_json::json!({"success": false, "message": e.to_string()}))),
    }
}

/// Get provider presets
async fn get_providers() -> Json<Vec<ProviderPreset>> {
    Json(get_provider_presets())
}

/// Chat request
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub context: Option<String>,
    pub session_id: Option<String>,
}

/// Chat response
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub reply: String,
    pub functions_called: Vec<String>,
}

/// Send a chat message
async fn chat(
    State(state): State<LlmState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    // Get enabled config
    let config = sqlx::query_as::<_, (i64, String, String, String, String, String, bool, chrono::NaiveDateTime, chrono::NaiveDateTime)>(
        "SELECT id, provider, display_name, api_base_url, api_key, model, enabled, created_at, updated_at FROM llm_config WHERE enabled = 1 LIMIT 1"
    )
    .fetch_optional(state.app_state.db.pool())
    .await
    .map_err(|e| internal_error(e))?;

    let config = match config {
        Some(c) => LlmConfig {
            id: c.0,
            provider: c.1,
            display_name: c.2,
            api_base_url: c.3,
            api_key: c.4,
            model: c.5,
            enabled: c.6,
            created_at: c.7,
            updated_at: c.8,
        },
        None => return Err(bad_request("未配置 LLM，请先在设置中配置")),
    };

    let registry = Arc::new(FunctionRegistry::new(state.app_state.clone()));
    let client = LlmClient::new(config, registry);

    // Build messages with optional context
    let mut messages = vec![
        ChatMessage {
            role: Role::System,
            content: Some(format!(
                "你是 FluxDNS 的 AI 助手，帮助用户管理 DNS 服务。{}",
                req.context.as_deref().unwrap_or("")
            )),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        },
    ];

    match client.process_message(&mut messages, req.message).await {
        Ok(reply) => Ok(Json(ChatResponse {
            reply,
            functions_called: vec![], // TODO: Track called functions
        })),
        Err(e) => {
            tracing::error!("Chat processing error: {:?}", e);
            // Return specific error message to frontend
            Err(ApiError {
                code: "LLM_ERROR".to_string(),
                message: e.to_string(),
                details: None,
            })
        }
    }
}

/// Streaming chat endpoint using SSE
async fn chat_stream(
    State(state): State<LlmState>,
    Json(req): Json<ChatRequest>,
) -> Response {
    // Quick check if LLM is configured
    let has_config = match sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM llm_config WHERE enabled = 1"
    )
    .fetch_one(state.app_state.db.pool())
    .await {
        Ok(count) => count > 0,
        Err(_) => false,
    };

    if !has_config {
        return Response::builder()
            .status(400)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(Body::from("data: {\"type\":\"error\",\"message\":\"未配置 LLM\"}\n\n"))
            .unwrap();
    }

    // Build messages with context
    let messages = vec![
        ChatMessage {
            role: Role::System,
            content: Some(format!(
                "你是 FluxDNS 的 AI 助手，帮助用户管理 DNS 服务。{}",
                req.context.as_deref().unwrap_or("")
            )),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        },
        ChatMessage {
            role: Role::User,
            content: Some(req.message.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        },
    ];

    // Create a channel for SSE events
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, std::io::Error>>(100);

    // Clone what we need for the async task
    let app_state = state.app_state.clone();
    let initial_messages = messages;
    let session_id = req.session_id.clone();
    let user_message = req.message;

    // Spawn a task to process the streaming response with tool call loop
    tokio::spawn(async move {
        let mut current_messages = initial_messages;
        let registry = FunctionRegistry::new(app_state.clone());
        
        // Save user message to database if session_id provided
        if let Some(ref sid) = session_id {
            let _ = sqlx::query(
                "INSERT INTO llm_messages (session_id, role, content, created_at) VALUES (?, 'user', ?, CURRENT_TIMESTAMP)"
            )
            .bind(sid)
            .bind(&user_message)
            .execute(app_state.db.pool())
            .await;
            
            // Update session updated_at
            let _ = sqlx::query("UPDATE llm_sessions SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind(sid)
                .execute(app_state.db.pool())
                .await;
        }
        
        // Accumulate full assistant response for saving
        let mut full_assistant_response = String::new();
        // Tool call loop - continue until we get a final response
        loop {
            // Get streaming response from LLM (non-streaming for follow-up calls with tools)
            let client = {
                let config = match sqlx::query_as::<_, (i64, String, String, String, String, String, bool, chrono::NaiveDateTime, chrono::NaiveDateTime)>(
                    "SELECT id, provider, display_name, api_base_url, api_key, model, enabled, created_at, updated_at FROM llm_config WHERE enabled = 1 LIMIT 1"
                )
                .fetch_optional(app_state.db.pool())
                .await {
                    Ok(Some(c)) => LlmConfig {
                        id: c.0,
                        provider: c.1,
                        display_name: c.2,
                        api_base_url: c.3,
                        api_key: c.4,
                        model: c.5,
                        enabled: c.6,
                        created_at: c.7,
                        updated_at: c.8,
                    },
                    _ => {
                        let event = StreamEvent::Error { message: "配置读取失败".to_string() };
                        let json = serde_json::to_string(&event).unwrap_or_default();
                        let _ = tx.send(Ok(format!("data: {}\n\n", json))).await;
                        return;
                    }
                };
                LlmClient::new(config, Arc::new(FunctionRegistry::new(app_state.clone())))
            };

            let llm_response = match client.send_stream_request(current_messages.clone()).await {
                Ok(resp) => resp,
                Err(e) => {
                    let event = StreamEvent::Error { message: e.to_string() };
                    let json = serde_json::to_string(&event).unwrap_or_default();
                    let _ = tx.send(Ok(format!("data: {}\n\n", json))).await;
                    return;
                }
            };

            let mut stream = llm_response.bytes_stream();
            let mut buffer = String::new();
            
            // Collect tool calls and content across chunks
            let mut accumulated_tool_calls: std::collections::HashMap<u32, (String, String, String)> = std::collections::HashMap::new();
            let mut has_tool_calls = false;
            let mut collected_content = String::new();
            let mut is_tool_call_finish = false;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        
                        while let Some(pos) = buffer.find("\n\n") {
                            let line = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();

                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    break;
                                }

                                if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) {
                                    if let Some(choice) = chunk.choices.first() {
                                        // Content delta
                                        if let Some(content) = &choice.delta.content {
                                            if !content.is_empty() {
                                                collected_content.push_str(content);
                                                full_assistant_response.push_str(content);
                                                let event = StreamEvent::Content { text: content.clone() };
                                                let json = serde_json::to_string(&event).unwrap_or_default();
                                                let _ = tx.send(Ok(format!("data: {}\n\n", json))).await;
                                            }
                                        }

                                        // Reasoning content
                                        if let Some(reasoning) = &choice.delta.reasoning_content {
                                            if !reasoning.is_empty() {
                                                let event = StreamEvent::Reasoning { text: reasoning.clone() };
                                                let json = serde_json::to_string(&event).unwrap_or_default();
                                                let _ = tx.send(Ok(format!("data: {}\n\n", json))).await;
                                            }
                                        }

                                        // Tool calls
                                        if let Some(tool_calls) = &choice.delta.tool_calls {
                                            has_tool_calls = true;
                                            for tc in tool_calls {
                                                let entry = accumulated_tool_calls.entry(tc.index).or_insert((
                                                    String::new(), String::new(), String::new(),
                                                ));
                                                if let Some(id) = &tc.id { entry.0 = id.clone(); }
                                                if let Some(func) = &tc.function {
                                                    if let Some(name) = &func.name { entry.1 = name.clone(); }
                                                    if let Some(args) = &func.arguments { entry.2.push_str(args); }
                                                }
                                            }
                                        }

                                        // Check finish reason
                                        if let Some(finish) = &choice.finish_reason {
                                            if finish == "tool_calls" {
                                                is_tool_call_finish = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let event = StreamEvent::Error { message: e.to_string() };
                        let json = serde_json::to_string(&event).unwrap_or_default();
                        let _ = tx.send(Ok(format!("data: {}\n\n", json))).await;
                        return;
                    }
                }
            }

            // If we have tool calls to execute
            if is_tool_call_finish && has_tool_calls && !accumulated_tool_calls.is_empty() {
                // Build assistant message with tool calls
                let tool_calls_vec: Vec<crate::llm::types::ToolCall> = accumulated_tool_calls.iter()
                    .map(|(_idx, (id, name, args))| crate::llm::types::ToolCall {
                        id: id.clone(),
                        call_type: "function".to_string(),
                        function: crate::llm::types::FunctionCall {
                            name: name.clone(),
                            arguments: args.clone(),
                        },
                    })
                    .collect();

                current_messages.push(ChatMessage {
                    role: Role::Assistant,
                    content: if collected_content.is_empty() { None } else { Some(collected_content.clone()) },
                    name: None,
                    tool_calls: Some(tool_calls_vec),
                    tool_call_id: None,
                    reasoning_content: None,
                });

                // Execute each tool and add result messages
                for (_idx, (id, name, args)) in &accumulated_tool_calls {
                    // Notify frontend
                    let tc_event = StreamEvent::ToolCall { name: name.clone(), arguments: args.clone() };
                    let json = serde_json::to_string(&tc_event).unwrap_or_default();
                    let _ = tx.send(Ok(format!("data: {}\n\n", json))).await;

                    // Execute tool
                    let result = registry.execute(name, args).await;
                    let result_json = serde_json::to_string(&result).unwrap_or_default();

                    // Notify frontend of result
                    let tr_event = StreamEvent::ToolResult { name: name.clone(), result: result_json.clone() };
                    let json = serde_json::to_string(&tr_event).unwrap_or_default();
                    let _ = tx.send(Ok(format!("data: {}\n\n", json))).await;

                    // Add tool result message
                    current_messages.push(ChatMessage {
                        role: Role::Tool,
                        content: Some(result_json),
                        name: Some(name.clone()),
                        tool_calls: None,
                        tool_call_id: Some(id.clone()),
                        reasoning_content: None,
                    });
                }

                // Continue the loop to get LLM's follow-up response
                continue;
            }

            // No more tool calls, we're done
            break;
        }

        // Save assistant response to database if session_id provided
        if let Some(ref sid) = session_id {
            if !full_assistant_response.is_empty() {
                let _ = sqlx::query(
                    "INSERT INTO llm_messages (session_id, role, content, created_at) VALUES (?, 'assistant', ?, CURRENT_TIMESTAMP)"
                )
                .bind(sid)
                .bind(&full_assistant_response)
                .execute(app_state.db.pool())
                .await;
            }
        }

        // Send done event
        let done_event = StreamEvent::Done;
        let json = serde_json::to_string(&done_event).unwrap_or_default();
        let _ = tx.send(Ok(format!("data: {}\n\n", json))).await;
    });

    // Create SSE response stream
    let stream = ReceiverStream::new(rx);
    let body = Body::from_stream(stream);

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(body)
        .unwrap()
}

/// Get all available tools (functions)
async fn get_tools(
    State(state): State<LlmState>,
) -> Result<Json<Vec<crate::llm::types::FunctionDefinition>>, ApiError> {
    let registry = FunctionRegistry::new(state.app_state.clone());
    let definitions = registry.get_tool_definitions()
        .into_iter()
        .map(|t| t.function)
        .collect();
    
    Ok(Json(definitions))
}

/// Get conversation history
async fn get_conversations(
    State(state): State<LlmState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let convos = sqlx::query_as::<_, (i64, String, String, Option<String>, chrono::NaiveDateTime)>(
        "SELECT id, session_id, role, content, created_at FROM llm_conversations ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(state.app_state.db.pool())
    .await
    .map_err(|e| internal_error(e))?;

    let result: Vec<serde_json::Value> = convos.into_iter().map(|(id, session, role, content, created)| {
        serde_json::json!({
            "id": id,
            "session_id": session,
            "role": role,
            "content": content,
            "created_at": created.to_string()
        })
    }).collect();

    Ok(Json(result))
}

/// Clear conversation history
async fn clear_conversations(
    State(state): State<LlmState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("DELETE FROM llm_conversations")
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    Ok(Json(serde_json::json!({"success": true, "message": "对话历史已清空"})))
}

// ============================================================================
// Session Management Handlers
// ============================================================================

/// Session response struct
#[derive(Debug, Serialize, Deserialize)]
struct SessionResponse {
    id: String,
    title: String,
    created_at: String,
    updated_at: String,
    message_count: i64,
}

/// Message response struct
#[derive(Debug, Serialize, Deserialize)]
struct MessageResponse {
    id: i64,
    role: String,
    content: Option<String>,
    tool_results: Option<String>,
    created_at: String,
}

/// Create session request
#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    title: Option<String>,
}

/// Update session title request
#[derive(Debug, Deserialize)]
struct UpdateTitleRequest {
    title: String,
}

/// Get all sessions
async fn get_sessions(
    State(state): State<LlmState>,
) -> Result<Json<Vec<SessionResponse>>, ApiError> {
    let sessions = sqlx::query_as::<_, (String, String, chrono::NaiveDateTime, chrono::NaiveDateTime)>(
        r#"
        SELECT s.id, s.title, s.created_at, s.updated_at
        FROM llm_sessions s
        ORDER BY s.updated_at DESC
        "#
    )
    .fetch_all(state.app_state.db.pool())
    .await
    .map_err(|e| internal_error(e))?;

    let mut result = Vec::new();
    for (id, title, created_at, updated_at) in sessions {
        // Get message count for each session
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM llm_messages WHERE session_id = ?")
            .bind(&id)
            .fetch_one(state.app_state.db.pool())
            .await
            .unwrap_or((0,));

        result.push(SessionResponse {
            id,
            title,
            created_at: created_at.to_string(),
            updated_at: updated_at.to_string(),
            message_count: count.0,
        });
    }

    Ok(Json(result))
}

/// Create a new session
async fn create_session(
    State(state): State<LlmState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();
    let title = req.title.unwrap_or_else(|| "新对话".to_string());
    let now = chrono::Utc::now().naive_utc();

    sqlx::query("INSERT INTO llm_sessions (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(&title)
        .bind(&now)
        .bind(&now)
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    Ok(Json(SessionResponse {
        id,
        title,
        created_at: now.to_string(),
        updated_at: now.to_string(),
        message_count: 0,
    }))
}

/// Get messages for a session
async fn get_session_messages(
    State(state): State<LlmState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Vec<MessageResponse>>, ApiError> {
    let messages = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, chrono::NaiveDateTime)>(
        "SELECT id, role, content, tool_results, created_at FROM llm_messages WHERE session_id = ? ORDER BY created_at ASC"
    )
    .bind(&id)
    .fetch_all(state.app_state.db.pool())
    .await
    .map_err(|e| internal_error(e))?;

    let result: Vec<MessageResponse> = messages.into_iter().map(|(id, role, content, tool_results, created_at)| {
        MessageResponse {
            id,
            role,
            content,
            tool_results,
            created_at: created_at.to_string(),
        }
    }).collect();

    Ok(Json(result))
}

/// Delete a session
async fn delete_session(
    State(state): State<LlmState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Delete messages first
    sqlx::query("DELETE FROM llm_messages WHERE session_id = ?")
        .bind(&id)
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    // Delete session
    sqlx::query("DELETE FROM llm_sessions WHERE id = ?")
        .bind(&id)
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// Update session title
async fn update_session_title(
    State(state): State<LlmState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<UpdateTitleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("UPDATE llm_sessions SET title = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&req.title)
        .bind(&id)
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// Response struct with masked API key
#[derive(Debug, Serialize)]
struct LlmConfigResponse {
    id: i64,
    provider: String,
    display_name: String,
    api_base_url: String,
    api_key: String,
    api_key_masked: String,
    model: String,
    enabled: bool,
}

/// Mask API key for display
fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        "********".to_string()
    } else {
        format!("{}...{}", &key[..4], &key[key.len()-4..])
    }
}
