# Rusk JSON-RPC Server Configuration

This document details the configuration options for the Rusk JSON-RPC server.

## Introduction

The JSON-RPC server provides an interface for interacting with the Rusk node. Its behavior can be customized through various configuration settings.

### Configuration Loading Precedence

Configuration values are loaded in the following order, with later sources overriding earlier ones:

1. **Default Values:** Sensible defaults are built into the server.
2. **TOML File:** Settings specified in the `[jsonrpc]` section of the `default.config.toml` file.
3. **Environment Variables:** Variables prefixed with `RUSK_JSONRPC_` override corresponding settings from the file or defaults.

### Configuration File

- **Default Location:** `default.config.toml` in the Rusk project root directory (usually where the main `Cargo.toml` resides).
- **Structure:** All JSON-RPC specific settings **must** be placed under the `[jsonrpc]` section within this file (e.g., `[jsonrpc.http]`, `[jsonrpc.rate_limit]`).
- **Override Location:** The path to the configuration file can be overridden by setting the `RUSK_JSONRPC_CONFIG_PATH` environment variable.

### Environment Variables

- **Prefix:** All environment variables related to the JSON-RPC server configuration start with `RUSK_JSONRPC_`.
- **Format:** Variables generally correspond to the nested TOML keys, using underscores (`_`) as separators (e.g., `RUSK_JSONRPC_HTTP_BIND_ADDRESS`).
- **Limitations:** Complex list structures (like method-specific rate limits) cannot be fully configured via environment variables; use the TOML file for these.

### Security Validation

The server performs validation checks on the loaded configuration to prevent common insecure settings (e.g., binding to public interfaces without rate limiting, overly permissive CORS). Warnings or errors will be generated if insecure configurations are detected.

## Configuration Sections

### HTTP Server (`[jsonrpc.http]`)

Controls the behavior of the HTTP transport for the JSON-RPC server.

---

**`bind_address`**

- **Description:** The socket address (IP and port) the HTTP server should listen on. Use `0.0.0.0` to listen on all available network interfaces (requires careful security consideration, especially regarding rate limiting and firewalls).
- **Type:** `String` (Socket Address, e.g., "127.0.0.1:8546", "[::1]:8546", "0.0.0.0:8546")
- **Default:** `"127.0.0.1:8546"`
- **Environment Variable:** `RUSK_JSONRPC_HTTP_BIND_ADDRESS`
- **Validation:** Must be a valid socket address.
- **Security:** Binding to public interfaces (`0.0.0.0` or public IPs) without enabling rate limiting is a security risk and will fail validation.
- **Example TOML:**

    ```toml
    # [jsonrpc.http]
    # bind_address = "127.0.0.1:8546"
    ```

---

## Full Configuration Examples

### Example 1: Local Development Setup

This setup prioritizes ease of use for local testing. It binds only to localhost and disables some security features like rate limiting and strict CORS. **Do not use this configuration in production.**

```toml
# In default.config.toml

[jsonrpc]
  [jsonrpc.http]
  bind_address = "127.0.0.1:8546"
  # Allow larger bodies for testing uploads, etc.
  max_body_size = 52428800 # 50 MB

  [jsonrpc.http.cors]
  # Allow requests from any origin during development
  enabled = true
  allowed_origins = [] # Allow all
  allowed_methods = ["POST", "GET", "OPTIONS"]
  allowed_headers = ["Content-Type", "Rusk-Version", "X-Custom-Test-Header"]
  allow_credentials = false # Keep false unless specifically needed

  [jsonrpc.ws]
  bind_address = "127.0.0.1:8547"

  [jsonrpc.rate_limit]
  # Disable rate limiting for easier testing
  enabled = false

  [jsonrpc.features]
  enable_websocket = true
  # Show detailed errors during development
  detailed_errors = true
  method_timing = true
  # Relax version checking for local clients
  strict_version_checking = false
  # Allow extra parameters during testing if needed (use with caution)
  strict_parameter_validation = true # Keep true unless absolutely necessary

  [jsonrpc.sanitization]
  # Disable sanitization to see raw errors during development
  enabled = false
```

### Example 2: Secure Production Setup

This configuration aims for security suitable for a server exposed to the internet. It binds to all interfaces, enables rate limiting, uses strict CORS, and enables sanitization.

```toml
# In default.config.toml

[jsonrpc]
  [jsonrpc.http]
  # Bind to all interfaces - ensure firewall rules are in place!
  bind_address = "0.0.0.0:8546"
  # Standard body size limit
  max_body_size = 10485760 # 10 MB
  # Standard timeout
  request_timeout = 30
  # Standard connection limit
  max_connections = 100
  # Provide paths for TLS certificate and key for HTTPS
  # cert = "/etc/ssl/certs/rusk_server.pem"
  # key = "/etc/ssl/private/rusk_server.key"

  [jsonrpc.http.cors]
  enabled = true
  # Only allow specific frontend origin(s)
  allowed_origins = ["https://your-frontend-app.com"]
  # Standard methods needed
  allowed_methods = ["POST", "GET"]
  # Standard headers + any required custom ones
  allowed_headers = ["Content-Type", "Rusk-Version"]
  # Keep false unless credentials are required AND origins are specific
  allow_credentials = false
  # Standard max age
  max_age_seconds = 86400

  [jsonrpc.ws]
  # Bind to all interfaces - ensure firewall rules are in place!
  bind_address = "0.0.0.0:8547"
  # Standard message size limit
  max_message_size = 1048576 # 1 MB
  # Standard connection limits
  max_connections = 50
  max_subscriptions_per_connection = 10
  # Standard timeouts
  idle_timeout = 300
  max_events_per_second = 100

  [jsonrpc.rate_limit]
  # Rate limiting MUST be enabled for public interfaces
  enabled = true
  # Default limit (adjust based on expected load)
  [jsonrpc.rate_limit.default_limit]
  requests = 100
  window = 60
  # WebSocket connection limit
  [jsonrpc.rate_limit.websocket_limit]
  requests = 10
  window = 60
  # Specific limits for common/heavy methods
  [[jsonrpc.rate_limit.method_limits]]
  method_pattern = "get*"
  limit = { requests = 200, window = 60 }
  [[jsonrpc.rate_limit.method_limits]]
  method_pattern = "prove"
  limit = { requests = 5, window = 60 } # Lower limit for heavy methods

  [jsonrpc.features]
  enable_websocket = true
  # Disable detailed errors in production
  detailed_errors = false
  # Enable timing for monitoring
  method_timing = true
  # Enforce version checking if desired
  strict_version_checking = true
  # Keep strict parameter validation enabled
  strict_parameter_validation = true
  # Standard block range limit
  max_block_range = 1000
  # Standard batch size limit
  max_batch_size = 20

  [jsonrpc.sanitization]
  # Keep sanitization enabled
  enabled = true
  # Ensure path sanitization is enabled
  sanitize_paths = true
  # Use default sensitive terms + add any custom ones
  # sensitive_terms = [ ... default terms ..., "internal_project_code"]
  max_message_length = 200
  redaction_marker = "[REDACTED]"
```

## Security Considerations

Configuring the JSON-RPC server securely is crucial, especially when exposing it to untrusted networks. The configuration loader includes validation checks (`validate()` method in `config.rs`) to prevent common insecure settings. Pay close attention to the following options:

- **Binding Address (`http.bind_address`, `ws.bind_address`):**
  - Binding to `0.0.0.0` or a public IP makes the server accessible from external networks.
  - **Recommendation:** Only bind to public interfaces if necessary. If you do, **ensure rate limiting is enabled** (`rate_limit.enabled = true`) and consider firewall rules. Binding to `127.0.0.1` (localhost) is generally safer for local-only access.
  - Validation fails if binding to a public interface without rate limiting enabled.

- **Rate Limiting (`rate_limit.*`):**
  - **Recommendation:** Keep rate limiting enabled (`rate_limit.enabled = true`) for all publicly exposed servers. Adjust `default_limit`, `websocket_limit`, and `method_limits` to reasonable values based on expected load and server capacity. Avoid excessively high limits.
  - Validation fails if rate limiting is disabled while binding to a public interface. Excessively high limits may also fail validation.

- **CORS (`http.cors.*`):**
  - **Recommendation:** Be as specific as possible with `allowed_origins`. Avoid using `[]` or `["*"]` (allow all) if possible.
  - Never set `allow_credentials = true` if `allowed_origins` permits all origins (`[]` or `["*"]`).
  - Validation fails for the combination of wildcard origin and `allow_credentials = true`.

- **Request/Message Sizes (`http.max_body_size`, `ws.max_message_size`):**
  - **Recommendation:** Keep these limits as low as practical for your expected use case to mitigate resource exhaustion attacks.
  - Validation fails if limits exceed safe maximums (e.g., 100MB for HTTP, 10MB for WS).

- **Sanitization (`sanitization.*`):**
  - **Recommendation:** Keep sanitization enabled (`sanitization.enabled = true`) and path sanitization enabled (`sanitization.sanitize_paths = true`), especially for public-facing servers, to prevent leaking sensitive data or file paths in error messages. Add application-specific sensitive terms to `sensitive_terms`.
  - Validation fails if sanitization or path sanitization is disabled while binding to a public interface. It also checks for a minimum number of sensitive terms.

- **Strict Parameter Validation (`features.strict_parameter_validation`):**
  - **Recommendation:** Keep enabled (`true`). Disabling it allows clients to send extra, potentially malicious, parameters that might be processed unexpectedly by handlers.
  - Validation fails if disabled.

- **TLS (`http.cert`, `http.key`):**
  - **Recommendation:** Use TLS (HTTPS) for all communication over untrusted networks by providing valid certificate and key files.

---

## Environment Variable Summary

The following table maps TOML configuration keys to their corresponding environment variables. Remember to prefix all variables with `RUSK_JSONRPC_`.

| TOML Key                                       | Environment Variable Suffix                 | Notes                                      |
| :--------------------------------------------- | :------------------------------------------ | :----------------------------------------- |
| `http.bind_address`                            | `HTTP_BIND_ADDRESS`                         |                                            |
| `http.max_body_size`                           | `HTTP_MAX_BODY_SIZE`                        |                                            |
| `http.request_timeout`                         | `HTTP_REQUEST_TIMEOUT_SECS`                 | Value in seconds                           |
| `http.max_connections`                         | `HTTP_MAX_CONNECTIONS`                      |                                            |
| `http.cert`                                    | `HTTP_CERT`                                 | File path                                  |
| `http.key`                                     | `HTTP_KEY`                                  | File path                                  |
| `http.cors.enabled`                            | `CORS_ENABLED`                              | `true` or `false`                          |
| `http.cors.allowed_origins`                    | `CORS_ALLOWED_ORIGINS`                      | Comma-separated list                       |
| `http.cors.allowed_methods`                    | `CORS_ALLOWED_METHODS`                      | Comma-separated list                       |
| `http.cors.allowed_headers`                    | `CORS_ALLOWED_HEADERS`                      | Comma-separated list                       |
| `http.cors.allow_credentials`                  | `CORS_ALLOW_CREDENTIALS`                    | `true` or `false`                          |
| `http.cors.max_age_seconds`                    | `CORS_MAX_AGE_SECONDS`                      | Value in seconds                           |
| `ws.bind_address`                              | `WS_BIND_ADDRESS`                           |                                            |
| `ws.max_message_size`                          | `WS_MAX_MESSAGE_SIZE`                       |                                            |
| `ws.max_connections`                           | `WS_MAX_CONNECTIONS`                        |                                            |
| `ws.max_subscriptions_per_connection`          | `WS_MAX_SUBSCRIPTIONS_PER_CONNECTION`       |                                            |
| `ws.idle_timeout`                              | `WS_IDLE_TIMEOUT_SECS`                      | Value in seconds                           |
| `ws.max_events_per_second`                     | `WS_MAX_EVENTS_PER_SECOND`                  |                                            |
| `rate_limit.enabled`                           | `RATE_LIMIT_ENABLED`                        | `true` or `false`                          |
| `rate_limit.default_limit.requests`            | `RATE_LIMIT_DEFAULT_REQUESTS`               |                                            |
| `rate_limit.default_limit.window`              | `RATE_LIMIT_DEFAULT_WINDOW_SECS`            | Value in seconds                           |
| `rate_limit.websocket_limit.requests`          | `RATE_LIMIT_WEBSOCKET_REQUESTS`             |                                            |
| `rate_limit.websocket_limit.window`            | `RATE_LIMIT_WEBSOCKET_WINDOW_SECS`          | Value in seconds                           |
| `rate_limit.method_limits`                     | -                                           | Configure via TOML file only               |
| `features.enable_websocket`                    | `FEATURE_ENABLE_WEBSOCKET`                  | `true` or `false`                          |
| `features.detailed_errors`                     | `FEATURE_DETAILED_ERRORS`                   | `true` or `false`                          |
| `features.method_timing`                       | `FEATURE_METHOD_TIMING`                     | `true` or `false`                          |
| `features.strict_version_checking`             | `FEATURE_STRICT_VERSION_CHECKING`           | `true` or `false`                          |
| `features.strict_parameter_validation`         | `FEATURE_STRICT_PARAMETER_VALIDATION`       | `true` or `false`                          |
| `features.max_block_range`                     | `FEATURE_MAX_BLOCK_RANGE`                   |                                            |
| `features.max_batch_size`                      | `FEATURE_MAX_BATCH_SIZE`                    |                                            |
| `sanitization.enabled`                         | `SANITIZATION_ENABLED`                      | `true` or `false`                          |
| `sanitization.sensitive_terms`                 | `SANITIZATION_SENSITIVE_TERMS`              | Comma-separated list                       |
| `sanitization.max_message_length`              | `SANITIZATION_MAX_MESSAGE_LENGTH`           |                                            |
| `sanitization.redaction_marker`                | `SANITIZATION_REDACTION_MARKER`             |                                            |
| `sanitization.sanitize_paths`                  | `SANITIZATION_SANITIZE_PATHS`               | `true` or `false`                          |

---

### Sanitization (`[jsonrpc.sanitization]`)

Controls the sanitization of error messages and potentially other responses to prevent leakage of sensitive information.

---

**`enabled`**

- **Description:** Enables or disables all message sanitization features.
- **Type:** `bool`
- **Default:** `true`
- **Environment Variable:** `RUSK_JSONRPC_SANITIZATION_ENABLED`
- **Security:** Disabling sanitization when binding to public interfaces is a security risk and will fail validation.
- **Example TOML:**

    ```toml
    # [jsonrpc.sanitization]
    # enabled = true
    ```

---

**`sensitive_terms`**

- **Description:** A list of case-insensitive terms that will be replaced with the `redaction_marker` in error messages. Add any application-specific sensitive keywords here.
- **Type:** `Vec<String>`
- **Default:** (List including "password", ".wallet", ".key", "secret", "private", "token", "api_key", "auth", "passphrase", "mnemonic", "seed", etc.)
- **Environment Variable:** `RUSK_JSONRPC_SANITIZATION_SENSITIVE_TERMS` (Comma-separated list)
- **Validation:** Security validation checks if a minimum number of terms are present when enabled.
- **Example TOML:**

    ```toml
    # [jsonrpc.sanitization]
    # sensitive_terms = ["password", "secret", "internal_id"]
    ```

---

**`max_message_length`**

- **Description:** The maximum length (in characters) an error message can have before being truncated and appended with "...".
- **Type:** `usize`
- **Default:** `200`
- **Environment Variable:** `RUSK_JSONRPC_SANITIZATION_MAX_MESSAGE_LENGTH`
- **Validation:** Must be greater than 0.
- **Example TOML:**

    ```toml
    # [jsonrpc.sanitization]
    # max_message_length = 200
    ```

---

**`redaction_marker`**

- **Description:** The placeholder string used to replace sensitive terms found in messages.
- **Type:** `String`
- **Default:** `"[REDACTED]"`
- **Environment Variable:** `RUSK_JSONRPC_SANITIZATION_REDACTION_MARKER`
- **Validation:** Must not be empty.
- **Example TOML:**

    ```toml
    # [jsonrpc.sanitization]
    # redaction_marker = "[SENSITIVE]"
    ```

---

**`sanitize_paths`**

- **Description:** If `true`, attempts to detect and sanitize file paths (both Windows and Unix-style) found in messages, replacing separators and potentially redacting paths containing sensitive terms.
- **Type:** `bool`
- **Default:** `true`
- **Environment Variable:** `RUSK_JSONRPC_SANITIZATION_SANITIZE_PATHS`
- **Security:** Path sanitization must be enabled if binding to public interfaces; disabling it will fail security validation in that case.
- **Example TOML:**

    ```toml
    # [jsonrpc.sanitization]
    # sanitize_paths = true
    ```

---

### Feature Toggles (`[jsonrpc.features]`)

Enables or disables specific server features and behaviors.

---

**`enable_websocket`**

- **Description:** Enables or disables the WebSocket server entirely. If disabled, the settings in the `[jsonrpc.ws]` section are ignored.
- **Type:** `bool`
- **Default:** `true`
- **Environment Variable:** `RUSK_JSONRPC_FEATURE_ENABLE_WEBSOCKET`
- **Example TOML:**

    ```toml
    # [jsonrpc.features]
    # enable_websocket = true
    ```

---

**`detailed_errors`**

- **Description:** Controls whether detailed error messages (potentially including internal context) are returned to the client. If `false`, more generic error messages are used.
- **Type:** `bool`
- **Default:** `true`
- **Environment Variable:** `RUSK_JSONRPC_FEATURE_DETAILED_ERRORS`
- **Security:** Consider setting to `false` in production environments to minimize information leakage, relying on server-side logs for debugging.
- **Example TOML:**

    ```toml
    # [jsonrpc.features]
    # detailed_errors = true
    ```

---

**`method_timing`**

- **Description:** Enables or disables the collection and potential logging/reporting of timing metrics for individual JSON-RPC method calls.
- **Type:** `bool`
- **Default:** `true`
- **Environment Variable:** `RUSK_JSONRPC_FEATURE_METHOD_TIMING`
- **Example TOML:**

    ```toml
    # [jsonrpc.features]
    # method_timing = true
    ```

---

**`strict_version_checking`**

- **Description:** If `true`, the server requires clients to send a compatible `Rusk-Version` HTTP header and will reject requests with missing or incompatible versions.
- **Type:** `bool`
- **Default:** `false`
- **Environment Variable:** `RUSK_JSONRPC_FEATURE_STRICT_VERSION_CHECKING`
- **Example TOML:**

    ```toml
    # [jsonrpc.features]
    # strict_version_checking = false
    ```

---

**`strict_parameter_validation`**

- **Description:** If `true`, requests with unexpected or extraneous parameters will be rejected with an `InvalidParams` error. If `false`, extra parameters might be ignored.
- **Type:** `bool`
- **Default:** `true`
- **Environment Variable:** `RUSK_JSONRPC_FEATURE_STRICT_PARAMETER_VALIDATION`
- **Security:** Keeping this enabled (`true`) is recommended to prevent potential vulnerabilities related to unexpected parameters. Disabling it will fail security validation.
- **Example TOML:**

    ```toml
    # [jsonrpc.features]
    # strict_parameter_validation = true
    ```

---

**`max_block_range`**

- **Description:** The maximum number of blocks allowed in methods that query a range of blocks (e.g., `getLogs`). Helps prevent resource exhaustion from overly broad queries.
- **Type:** `u64`
- **Default:** `1000`
- **Environment Variable:** `RUSK_JSONRPC_FEATURE_MAX_BLOCK_RANGE`
- **Validation:** Must be greater than 0. Excessively high values may fail security validation.
- **Example TOML:**

    ```toml
    # [jsonrpc.features]
    # max_block_range = 1000
    ```

---

**`max_batch_size`**

- **Description:** The maximum number of individual requests allowed within a single JSON-RPC batch request array.
- **Type:** `usize`
- **Default:** `20`
- **Environment Variable:** `RUSK_JSONRPC_FEATURE_MAX_BATCH_SIZE`
- **Validation:** Must be greater than 0.
- **Example TOML:**

    ```toml
    # [jsonrpc.features]
    # max_batch_size = 20
    ```

---

### Rate Limiting (`[jsonrpc.rate_limit]`)

Controls request rate limiting to prevent abuse and ensure fair usage.

---

**`enabled`**

- **Description:** Enables or disables all rate limiting features (HTTP and WebSocket connection limits).
- **Type:** `bool`
- **Default:** `true`
- **Environment Variable:** `RUSK_JSONRPC_RATE_LIMIT_ENABLED`
- **Security:** Disabling rate limiting on servers exposed to untrusted networks is a significant security risk and will fail validation if bound to a public interface.
- **Example TOML:**

    ```toml
    # [jsonrpc.rate_limit]
    # enabled = true
    ```

---

#### Default Limit (`[jsonrpc.rate_limit.default_limit]`)

Applies to all incoming HTTP requests unless overridden by a method-specific limit.

**`requests`**

- **Description:** The maximum number of requests allowed within the specified time window.
- **Type:** `u64`
- **Default:** `100`
- **Environment Variable:** `RUSK_JSONRPC_RATE_LIMIT_DEFAULT_REQUESTS`
- **Validation:** Must be greater than 0. Excessively high values may fail security validation.
- **Example TOML:**

    ```toml
    # [jsonrpc.rate_limit.default_limit]
    # requests = 100
    ```

**`window`**

- **Description:** The time window (in seconds) for the `requests` limit.
- **Type:** `u64` (Seconds)
- **Default:** `60` (1 minute)
- **Environment Variable:** `RUSK_JSONRPC_RATE_LIMIT_DEFAULT_WINDOW_SECS`
- **Validation:** Must be greater than 0.
- **Example TOML:**

    ```toml
    # [jsonrpc.rate_limit.default_limit]
    # window = 60
    ```

---

#### WebSocket Connection Limit (`[jsonrpc.rate_limit.websocket_limit]`)

Limits the rate of *new* WebSocket connection attempts.

**`requests`**

- **Description:** The maximum number of new WebSocket connection attempts allowed within the specified time window.
- **Type:** `u64`
- **Default:** `10`
- **Environment Variable:** `RUSK_JSONRPC_RATE_LIMIT_WEBSOCKET_REQUESTS`
- **Validation:** Must be greater than 0.
- **Example TOML:**

    ```toml
    # [jsonrpc.rate_limit.websocket_limit]
    # requests = 10
    ```

**`window`**

- **Description:** The time window (in seconds) for the new connection limit.
- **Type:** `u64` (Seconds)
- **Default:** `60` (1 minute)
- **Environment Variable:** `RUSK_JSONRPC_RATE_LIMIT_WEBSOCKET_WINDOW_SECS`
- **Validation:** Must be greater than 0.
- **Example TOML:**

    ```toml
    # [jsonrpc.rate_limit.websocket_limit]
    # window = 60
    ```

---

#### Method-Specific Limits (`[[jsonrpc.rate_limit.method_limits]]`)

Allows overriding the default rate limit for specific JSON-RPC methods or groups of methods using wildcard patterns. Defined as an array of tables.

**`method_pattern`**

- **Description:** A pattern to match against JSON-RPC method names. Supports `*` as a wildcard (e.g., `"get*"`, `"eth_sendTransaction"`, `"*"`).
- **Type:** `String`
- **Default:** `""` (Not practically useful as a default, must be specified)
- **Environment Variable:** Not directly configurable via flat env vars. Use the TOML file.
- **Validation:** Must not be empty.
- **Example TOML:**

    ```toml
    # [[jsonrpc.rate_limit.method_limits]]
    # method_pattern = "get*"
    # limit = { requests = 200, window = 60 }
    ```

**`limit`**

- **Description:** The rate limit (`requests` and `window`) to apply to methods matching the `method_pattern`.
- **Type:** Table containing `requests` (u64) and `window` (u64, seconds).
- **Default:** `{ requests = 100, window = 60 }` (Inherits defaults from `RateLimit`)
- **Environment Variable:** Not directly configurable via flat env vars. Use the TOML file.
- **Validation:** `requests` and `window` must be greater than 0. Excessively high values may fail security validation.
- **Example TOML:**

    ```toml
    # [[jsonrpc.rate_limit.method_limits]]
    # method_pattern = "prove"
    # limit = { requests = 10, window = 60 }
    ```

---

### WebSocket Server (`[jsonrpc.ws]`)

Controls the behavior of the WebSocket transport, primarily used for subscriptions and receiving real-time updates. This section is only relevant if `features.enable_websocket` is `true`.

---

**`bind_address`**

- **Description:** The socket address (IP and port) the WebSocket server should listen on. Use `0.0.0.0` to listen on all available network interfaces (requires careful security consideration).
- **Type:** `String` (Socket Address, e.g., "127.0.0.1:8547", "[::1]:8547", "0.0.0.0:8547")
- **Default:** `"127.0.0.1:8547"`
- **Environment Variable:** `RUSK_JSONRPC_WS_BIND_ADDRESS`
- **Validation:** Must be a valid socket address.
- **Security:** Binding to public interfaces (`0.0.0.0` or public IPs) without enabling rate limiting is a security risk and will fail validation.
- **Example TOML:**

    ```toml
    # [jsonrpc.ws]
    # bind_address = "127.0.0.1:8547"
    ```

---

**`max_message_size`**

- **Description:** The maximum allowed size (in bytes) for incoming WebSocket messages.
- **Type:** `usize` (Bytes)
- **Default:** `1048576` (1 MB)
- **Environment Variable:** `RUSK_JSONRPC_WS_MAX_MESSAGE_SIZE`
- **Validation:** Must be greater than 0.
- **Security:** Setting this too high can expose the server to resource exhaustion. Values above 10MB will trigger a security validation error.
- **Example TOML:**

    ```toml
    # [jsonrpc.ws]
    # max_message_size = 1048576
    ```

---

**`max_connections`**

- **Description:** The maximum number of concurrent WebSocket connections the server will accept.
- **Type:** `usize`
- **Default:** `50`
- **Environment Variable:** `RUSK_JSONRPC_WS_MAX_CONNECTIONS`
- **Validation:** Must be greater than 0.
- **Example TOML:**

    ```toml
    # [jsonrpc.ws]
    # max_connections = 50
    ```

---

**`max_subscriptions_per_connection`**

- **Description:** The maximum number of active subscriptions allowed per single WebSocket connection.
- **Type:** `usize`
- **Default:** `10`
- **Environment Variable:** `RUSK_JSONRPC_WS_MAX_SUBSCRIPTIONS_PER_CONNECTION`
- **Validation:** Must be greater than 0.
- **Example TOML:**

    ```toml
    # [jsonrpc.ws]
    # max_subscriptions_per_connection = 10
    ```

---

**`idle_timeout`**

- **Description:** The maximum time (in seconds) a WebSocket connection can remain idle (no messages received) before being closed by the server.
- **Type:** `u64` (Seconds)
- **Default:** `300` (5 minutes)
- **Environment Variable:** `RUSK_JSONRPC_WS_IDLE_TIMEOUT_SECS`
- **Validation:** Must be greater than 0.
- **Example TOML:**

    ```toml
    # [jsonrpc.ws]
    # idle_timeout = 300
    ```

---

**`max_events_per_second`**

- **Description:** The maximum number of events (messages) the server will attempt to send per second over a single WebSocket connection. This acts as a basic form of outbound rate limiting to prevent overwhelming a single client.
- **Type:** `usize`
- **Default:** `100`
- **Environment Variable:** `RUSK_JSONRPC_WS_MAX_EVENTS_PER_SECOND`
- **Validation:** Must be greater than 0.
- **Example TOML:**

    ```toml
    # [jsonrpc.ws]
    # max_events_per_second = 100
    ```

---

**`max_body_size`**

- **Description:** The maximum allowed size (in bytes) for incoming HTTP request bodies. This helps prevent denial-of-service attacks using large requests.
- **Type:** `usize` (Bytes)
- **Default:** `10485760` (10 MB)
- **Environment Variable:** `RUSK_JSONRPC_HTTP_MAX_BODY_SIZE`
- **Validation:** Must be greater than 0.
- **Security:** Setting this too high can expose the server to resource exhaustion attacks. Values above 100MB will trigger a security validation error.
- **Example TOML:**

    ```toml
    # [jsonrpc.http]
    # max_body_size = 10485760
    ```

---

**`request_timeout`**

- **Description:** The maximum time (in seconds) the server will wait for a single HTTP request to complete before timing out.
- **Type:** `u64` (Seconds)
- **Default:** `30`
- **Environment Variable:** `RUSK_JSONRPC_HTTP_REQUEST_TIMEOUT_SECS`
- **Validation:** Must be greater than 0.
- **Example TOML:**

    ```toml
    # [jsonrpc.http]
    # request_timeout = 30
    ```

---

**`max_connections`**

- **Description:** The maximum number of concurrent HTTP connections the server will accept.
- **Type:** `usize`
- **Default:** `100`
- **Environment Variable:** `RUSK_JSONRPC_HTTP_MAX_CONNECTIONS`
- **Validation:** Must be greater than 0.
- **Example TOML:**

    ```toml
    # [jsonrpc.http]
    # max_connections = 100
    ```

---

**`cert`**

- **Description:** Optional path to the TLS certificate file (PEM format) to enable HTTPS. If specified, `key` must also be provided.
- **Type:** `String` (File Path)
- **Default:** `None`
- **Environment Variable:** `RUSK_JSONRPC_HTTP_CERT`
- **Validation:** If set, the file must exist and be readable. Must be set alongside `key`.
- **Example TOML:**

    ```toml
    # [jsonrpc.http]
    # cert = "/path/to/your/certificate.pem"
    ```

---

**`key`**

- **Description:** Optional path to the TLS private key file (PEM format) to enable HTTPS. If specified, `cert` must also be provided.
- **Type:** `String` (File Path)
- **Default:** `None`
- **Environment Variable:** `RUSK_JSONRPC_HTTP_KEY`
- **Validation:** If set, the file must exist and be readable. Must be set alongside `cert`.
- **Example TOML:**

    ```toml
    # [jsonrpc.http]
    # key = "/path/to/your/private_key.pem"
    ```

---

### CORS (`[jsonrpc.http.cors]`)

Controls Cross-Origin Resource Sharing (CORS) settings for the HTTP server, determining which web pages from different domains can access the JSON-RPC API.

---

**`enabled`**

- **Description:** Enables or disables CORS header processing. If disabled, no CORS headers are sent, and browsers will likely block cross-origin requests.
- **Type:** `bool`
- **Default:** `true`
- **Environment Variable:** `RUSK_JSONRPC_CORS_ENABLED`
- **Example TOML:**

    ```toml
    # [jsonrpc.http.cors]
    # enabled = true
    ```

---

**`allowed_origins`**

- **Description:** A list of allowed origin domains. If the list is empty, all origins are allowed (`*`). You can specify exact domains (e.g., `"https://example.com"`) or use a single wildcard `"*"` (use with caution).
- **Type:** `Vec<String>`
- **Default:** `[]` (Allow all origins)
- **Environment Variable:** `RUSK_JSONRPC_CORS_ALLOWED_ORIGINS` (Comma-separated list, e.g., `"https://app.example.com,https://test.example.com"`)
- **Security:** Allowing all origins (`[]` or `["*"]`) combined with `allow_credentials = true` is a significant security risk and will fail validation. Be as specific as possible.
- **Example TOML:**

    ```toml
    # [jsonrpc.http.cors]
    # allowed_origins = ["https://example.com", "https://test.com"]
    # allowed_origins = [] # Allow all
    ```

---

**`allowed_methods`**

- **Description:** A list of allowed HTTP methods for cross-origin requests (e.g., "POST", "GET", "OPTIONS").
- **Type:** `Vec<String>`
- **Default:** `["POST", "GET"]`
- **Environment Variable:** `RUSK_JSONRPC_CORS_ALLOWED_METHODS` (Comma-separated list)
- **Example TOML:**

    ```toml
    # [jsonrpc.http.cors]
    # allowed_methods = ["POST", "GET"]
    ```

---

**`allowed_headers`**

- **Description:** A list of allowed HTTP headers for cross-origin requests. Headers like `Content-Type` are usually required. Include any custom headers your clients might send (e.g., `Authorization`, `X-Request-ID`, `Rusk-Version`).
- **Type:** `Vec<String>`
- **Default:** `["Content-Type", "Rusk-Version"]`
- **Environment Variable:** `RUSK_JSONRPC_CORS_ALLOWED_HEADERS` (Comma-separated list)
- **Example TOML:**

    ```toml
    # [jsonrpc.http.cors]
    # allowed_headers = ["Content-Type", "Rusk-Version", "Authorization"]
    ```

---

**`allow_credentials`**

- **Description:** Controls whether the browser should include credentials (like cookies or HTTP authentication) in cross-origin requests.
- **Type:** `bool`
- **Default:** `false`
- **Environment Variable:** `RUSK_JSONRPC_CORS_ALLOW_CREDENTIALS`
- **Security:** Setting this to `true` with a wildcard `allowed_origins` is insecure and will fail validation. Only enable if you specifically need credentialed requests from known origins.
- **Example TOML:**

    ```toml
    # [jsonrpc.http.cors]
    # allow_credentials = false
    ```

---

**`max_age_seconds`**

- **Description:** Specifies how long (in seconds) the results of a preflight request (OPTIONS) can be cached by the browser.
- **Type:** `u64` (Seconds)
- **Default:** `86400` (24 hours)
- **Environment Variable:** `RUSK_JSONRPC_CORS_MAX_AGE_SECONDS`
- **Example TOML:**

    ```toml
    # [jsonrpc.http.cors]
    # max_age_seconds = 86400

  ```

---
