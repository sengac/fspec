// PROV-094: opt-in scripted Codex (ChatGPT subscription) provider.
//
// Parity target: the built-in CodexProvider (rust/providers/src/codex/).
// Device authorization against the OpenAI issuer, form-encoded token
// exchange and refresh, the ChatGPT account id from the token JWT, and
// requests to the ChatGPT Responses endpoint. Tokens are the provider's
// stored OAuth tokens (cred::read_tokens). Every failure throws — nothing
// is silently substituted.

// ---------------------------------------------------------------- helpers

fn form(fields) {
    let pairs = [];
    for key in fields.keys() {
        pairs.push(key + "=" + oauth::urlencoded(fields[key]));
    }
    pairs.reduce(|acc, pair| if acc == () { pair } else { acc + "&" + pair })
}

fn form_headers() {
    #{ "Content-Type": "application/x-www-form-urlencoded" }
}

// The claims of a JWT (its base64url payload segment).
fn jwt_claims(token) {
    let segments = token.split(".");
    if segments.len() < 2 {
        throw "Codex token is not a JWT";
    }
    json::parse(crypto::base64url_decode(segments[1]))
}

// ChatGPT account id: top-level claim, then the OpenAI auth claim, then
// the first organization — id_token first, then access_token (built-in
// codex_oauth.rs extract_account_id order).
fn account_from_claims(claims) {
    if type_of(claims.chatgpt_account_id) == "string" {
        return claims.chatgpt_account_id;
    }
    let auth = claims["https://api.openai.com/auth"];
    if type_of(auth) == "map" && type_of(auth.chatgpt_account_id) == "string" {
        return auth.chatgpt_account_id;
    }
    if type_of(claims.organizations) == "array" && claims.organizations.len() > 0 {
        let first = claims.organizations[0];
        if type_of(first) == "map" && type_of(first.id) == "string" {
            return first.id;
        }
    }
    ()
}

fn account_id(tokens) {
    for name in ["id_token", "access_token"] {
        let token = tokens[name];
        if type_of(token) == "string" && token.contains(".") {
            let found = account_from_claims(jwt_claims(token));
            if found != () {
                return found;
            }
        }
    }
    throw "Codex tokens carry no ChatGPT account id";
}

// Token map stored after an exchange or refresh. A response without
// expires_in uses the built-in provider's 3600 s default
// (codex/refreshing_client.rs DEFAULT_EXPIRY_SECS).
fn stored_tokens(response, previous_refresh) {
    let tokens = json::parse(response.body);
    if type_of(tokens.access_token) != "string" {
        throw "Codex token response has no access_token";
    }
    let refresh = if type_of(tokens.refresh_token) == "string" { tokens.refresh_token } else { previous_refresh };
    if type_of(refresh) != "string" {
        throw "Codex token response has no refresh_token";
    }
    let lifetime = if type_of(tokens.expires_in) == "i64" { tokens.expires_in } else { 3600 };
    let result = #{
        access_token: tokens.access_token,
        refresh_token: refresh,
        expires_at: time::now_secs() + lifetime,
    };
    if type_of(tokens.id_token) == "string" {
        result.id_token = tokens.id_token;
    }
    result.account_id = account_id(result);
    result
}

// ------------------------------------------------------------------ OAuth

fn auth_start(config) {
    let response = http::post(
        config.auth_url + "/api/accounts/deviceauth/usercode",
        form(#{ client_id: config.client_id }),
        form_headers(),
    );
    let device = json::parse(response.body);
    if type_of(device.user_code) != "string" || type_of(device.device_auth_id) != "string" {
        throw "Codex device authorization returned no user code";
    }
    #{
        device_auth_id: device.device_auth_id,
        user_code: device.user_code,
        interval: device.interval,
        verification_uri: config.auth_url + "/codex/device",
    }
}

// Poll the device authorization: 403/404 or authorization_pending mean
// "not yet"; slow_down asks for a longer interval; an authorization code
// is exchanged for tokens.
fn poll_for_token(config, device) {
    let response = http::post_status(
        config.auth_url + "/api/accounts/deviceauth/token",
        json::stringify(#{ device_auth_id: device.device_auth_id, user_code: device.user_code }),
        #{ "Content-Type": "application/json" },
    );
    if response.status == 403 || response.status == 404 {
        return #{ status: "pending" };
    }
    if response.status >= 400 {
        throw `Codex device polling failed (${response.status}): ${response.body}`;
    }
    let answer = json::parse(response.body);
    if type_of(answer.error) == "string" {
        if answer.error == "authorization_pending" {
            return #{ status: "pending" };
        }
        if answer.error == "slow_down" {
            return #{ status: "slow_down" };
        }
        throw `Codex device authorization failed: ${answer.error}`;
    }
    if type_of(answer.authorization_code) != "string" {
        throw "Codex device authorization returned no authorization code";
    }
    let exchanged = http::post(
        config.token_url,
        form(#{
            grant_type: "authorization_code",
            code: answer.authorization_code,
            code_verifier: answer.code_verifier,
            client_id: config.client_id,
            redirect_uri: config.auth_url + "/deviceauth/callback",
        }),
        form_headers(),
    );
    let tokens = stored_tokens(exchanged, ());
    tokens.status = "success";
    tokens
}

// Refresh 30 s before expiry (built-in EXPIRY_BUFFER_SECS). Tokens that
// carry no expiry are not refreshed proactively.
fn auth_needs_refresh(tokens) {
    if type_of(tokens.expires_at) != "i64" {
        return false;
    }
    tokens.expires_at <= time::now_secs() + 30
}

fn auth_refresh(config, tokens) {
    let response = http::post(
        config.token_url,
        form(#{
            grant_type: "refresh_token",
            client_id: config.client_id,
            refresh_token: tokens.refresh_token,
        }),
        form_headers(),
    );
    stored_tokens(response, tokens.refresh_token)
}

// --------------------------------------------------------------- requests

fn build_url(config) {
    config.base_url + "/responses"
}

fn build_headers(config) {
    let tokens = cred::read_tokens(config.name);
    if tokens == () {
        throw "Codex is not signed in: run the Codex device login first";
    }
    #{
        "authorization": "Bearer " + tokens.access_token,
        "ChatGPT-Account-Id": tokens.account_id,
        "originator": "rust",
        "Content-Type": "application/json",
    }
}

fn text_of(content) {
    if type_of(content) == "string" {
        return content;
    }
    let text = "";
    for part in content {
        if part.type == "text" {
            text += part.text;
        }
    }
    text
}

// System messages become `instructions`; without one, the configured base
// instructions (system_prompt.prefix) are used, as the built-in provider
// does (codex/completion.rs).
fn instructions(request) {
    let parts = [];
    for message in request.messages {
        if message.role == "system" {
            parts.push(text_of(message.content));
        }
    }
    if parts.len() > 0 {
        return parts.reduce(|acc, part| if acc == () { part } else { acc + "\n\n" + part });
    }
    if request.system_prefix == () {
        throw "Codex request has no instructions: no system message and no system_prompt.prefix";
    }
    request.system_prefix
}

fn image_item(source) {
    if source.type == "url" {
        return #{ type: "input_image", image_url: source.url };
    }
    #{ type: "input_image", image_url: "data:" + source.media_type + ";base64," + source.data }
}

fn input_items(messages) {
    let items = [];
    for message in messages {
        if message.role == "system" {
            continue;
        }
        let text_type = if message.role == "assistant" { "output_text" } else { "input_text" };
        if type_of(message.content) == "string" {
            items.push(#{ role: message.role, content: [#{ type: text_type, text: message.content }] });
            continue;
        }
        let pending = [];
        for part in message.content {
            if part.type == "text" {
                pending.push(#{ type: text_type, text: part.text });
            } else if part.type == "image" {
                pending.push(image_item(part.source));
            } else {
                if pending.len() > 0 {
                    items.push(#{ role: message.role, content: pending });
                    pending = [];
                }
                if part.type == "tool_use" {
                    items.push(#{
                        type: "function_call",
                        call_id: part.id,
                        name: part.name,
                        arguments: json::stringify(part.input),
                    });
                } else if part.type == "tool_result" {
                    items.push(#{ type: "function_call_output", call_id: part.tool_use_id, output: part.content });
                } else {
                    throw `Codex cannot send a ${part.type} content part`;
                }
            }
        }
        if pending.len() > 0 {
            items.push(#{ role: message.role, content: pending });
        }
    }
    items
}

fn build_request(request) {
    let body = #{
        model: request.model,
        instructions: instructions(request),
        store: false,
        input: input_items(request.messages),
    };
    if request.tools.len() > 0 {
        body.tools = request.tools.map(|tool| #{
            type: "function",
            name: tool.name,
            description: tool.description,
            parameters: tool.input_schema,
        });
    }
    // MODEL-010: the script receives the turn's level and the model's
    // capability and owns the wire. Off is the Responses API effort "none".
    if type_of(request.thinking_config) == "map" {
        let level = request.thinking_config.level;
        if type_of(level) != "string" {
            throw `Codex takes a named reasoning effort, not ${level}`;
        }
        let effort = if level == "off" { "none" } else { level };
        body.reasoning = #{ effort: effort, summary: "auto" };
    }
    body
}

fn build_stream_request(request) {
    let body = build_request(request);
    body.stream = true;
    body
}

// --------------------------------------------------------------- responses

fn usage_of(usage) {
    let result = #{ kind: "usage", input_tokens: usage.input_tokens, output_tokens: usage.output_tokens };
    if type_of(usage.output_tokens_details) == "map" && type_of(usage.output_tokens_details.reasoning_tokens) == "i64" {
        result.reasoning_tokens = usage.output_tokens_details.reasoning_tokens;
    }
    if type_of(usage.input_tokens_details) == "map" && type_of(usage.input_tokens_details.cached_tokens) == "i64" {
        result.cache_read_input_tokens = usage.input_tokens_details.cached_tokens;
    }
    result
}

fn failure_message(response) {
    if type_of(response) == "map" && type_of(response.error) == "map" && type_of(response.error.message) == "string" {
        return response.error.message;
    }
    "Codex response failed"
}

fn parse_response(raw) {
    if raw.status == "failed" {
        throw failure_message(raw);
    }
    let content = [];
    let used_tool = false;
    for item in raw.output {
        if item.type == "message" {
            for part in item.content {
                if part.type == "output_text" {
                    content.push(#{ type: "text", text: part.text });
                }
            }
        } else if item.type == "function_call" {
            used_tool = true;
            content.push(#{ type: "tool_use", id: item.call_id, name: item.name, input: json::parse(item.arguments) });
        }
    }
    let result = #{ content: content, stop_reason: if used_tool { "tool_use" } else { "end_turn" } };
    if type_of(raw.usage) == "map" {
        result.usage = usage_of(raw.usage);
    }
    result
}

fn parse_stream_chunk(config, data) {
    let event = json::parse(data);
    let kind = event.type;
    if kind == "response.output_text.delta" {
        return #{ kind: "text_delta", text: event.delta };
    }
    if kind == "response.reasoning_summary_text.delta" {
        return #{ kind: "reasoning_delta", text: event.delta };
    }
    if kind == "response.output_item.done" && event.item.type == "function_call" {
        return #{
            kind: "tool_call",
            id: event.item.call_id,
            name: event.item.name,
            arguments: event.item.arguments,
            complete: true,
        };
    }
    if kind == "response.completed" {
        let used_tool = false;
        if type_of(event.response.output) == "array" {
            for item in event.response.output {
                if item.type == "function_call" {
                    used_tool = true;
                }
            }
        }
        let chunks = [];
        if type_of(event.response.usage) == "map" {
            chunks.push(usage_of(event.response.usage));
        }
        chunks.push(#{ kind: "stop", reason: if used_tool { "tool_use" } else { "end_turn" } });
        return chunks;
    }
    if kind == "response.incomplete" {
        return #{ kind: "stop", reason: "max_tokens" };
    }
    if kind == "response.failed" || kind == "error" {
        throw failure_message(if kind == "error" { #{ error: event } } else { event.response });
    }
    #{ kind: "ignore" }
}

fn map_error(status, body) {
    let kind = if status == 401 || status == 403 {
        "auth"
    } else if status == 429 {
        "rate_limit"
    } else {
        "api"
    };
    #{ type: kind, message: `Codex request failed (${status}): ${body}` }
}
