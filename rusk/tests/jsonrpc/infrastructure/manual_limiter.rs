// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Unit tests for the `ManualRateLimiters` struct.

use rusk::jsonrpc::config::{
    ConfigError, MethodRateLimit, RateLimit, RateLimitConfig,
};
use rusk::jsonrpc::infrastructure::client_info::ClientInfo;
use rusk::jsonrpc::infrastructure::error::RateLimitError;
use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

const TEST_IP_1: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const TEST_IP_2: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
const TEST_PORT: u16 = 12345;

/// Helper to create a basic ClientInfo.
fn test_client_info(ip: IpAddr) -> ClientInfo {
    ClientInfo::new(SocketAddr::new(ip, TEST_PORT))
}

#[test]
fn test_new_manual_limiter_success() {
    let config = Arc::new(RateLimitConfig {
        enabled: true,
        websocket_limit: RateLimit {
            requests: 5,
            window: Duration::from_secs(1),
        },
        method_limits: vec![MethodRateLimit {
            method_pattern: "get*".to_string(),
            limit: RateLimit {
                requests: 10,
                window: Duration::from_secs(1),
            },
        }],
        default_limit: RateLimit::default(),
    });
    let result = ManualRateLimiters::new(config);
    assert!(result.is_ok());
}

#[test]
fn test_new_manual_limiter_disabled() {
    let config = Arc::new(RateLimitConfig {
        enabled: false,
        websocket_limit: RateLimit {
            requests: 5,
            window: Duration::from_secs(1),
        },
        method_limits: vec![MethodRateLimit {
            method_pattern: "get*".to_string(),
            limit: RateLimit {
                requests: 10,
                window: Duration::from_secs(1),
            },
        }],
        default_limit: RateLimit::default(),
    });
    let result = ManualRateLimiters::new(config);
    assert!(result.is_ok());
}

#[test]
fn test_new_manual_limiter_invalid_websocket_limit_req() {
    let config = Arc::new(RateLimitConfig {
        enabled: true,
        websocket_limit: RateLimit {
            requests: 0,
            window: Duration::from_secs(1),
        },
        method_limits: vec![],
        default_limit: RateLimit::default(),
    });
    let result = ManualRateLimiters::new(config);
    assert!(matches!(result, Err(ConfigError::Validation(_))));
}

#[test]
fn test_new_manual_limiter_invalid_websocket_limit_win() {
    let config = Arc::new(RateLimitConfig {
        enabled: true,
        websocket_limit: RateLimit {
            requests: 5,
            window: Duration::ZERO,
        },
        method_limits: vec![],
        default_limit: RateLimit::default(),
    });
    let result = ManualRateLimiters::new(config);
    assert!(matches!(result, Err(ConfigError::Validation(_))));
}

#[test]
fn test_new_manual_limiter_invalid_method_limit_req() {
    let config = Arc::new(RateLimitConfig {
        enabled: true,
        websocket_limit: RateLimit {
            requests: 10,
            window: Duration::from_secs(1),
        },
        method_limits: vec![MethodRateLimit {
            method_pattern: "get*".to_string(),
            limit: RateLimit {
                requests: 0,
                window: Duration::from_secs(1),
            },
        }],
        default_limit: RateLimit::default(),
    });
    let result = ManualRateLimiters::new(config);
    assert!(matches!(result, Err(ConfigError::Validation(_))));
}

#[test]
fn test_new_manual_limiter_invalid_method_limit_win() {
    let config = Arc::new(RateLimitConfig {
        enabled: true,
        websocket_limit: RateLimit {
            requests: 10,
            window: Duration::from_secs(1),
        },
        method_limits: vec![MethodRateLimit {
            method_pattern: "get*".to_string(),
            limit: RateLimit {
                requests: 5,
                window: Duration::ZERO,
            },
        }],
        default_limit: RateLimit::default(),
    });
    let result = ManualRateLimiters::new(config);
    assert!(matches!(result, Err(ConfigError::Validation(_))));
}

#[test]
fn test_new_manual_limiter_invalid_glob_pattern() {
    let config = Arc::new(RateLimitConfig {
        enabled: true,
        websocket_limit: RateLimit {
            requests: 10,
            window: Duration::from_secs(1),
        },
        method_limits: vec![MethodRateLimit {
            method_pattern: "[invalid".to_string(),
            limit: RateLimit {
                requests: 5,
                window: Duration::from_secs(1),
            },
        }],
        default_limit: RateLimit::default(),
    });
    let result = ManualRateLimiters::new(config);
    assert!(matches!(result, Err(ConfigError::Validation(_))));
    if let Err(ConfigError::Validation(msg)) = result {
        assert!(msg.contains("Invalid glob pattern"));
    }
}

// --- Tests for check_websocket_limit ---

#[tokio::test]
async fn test_check_websocket_limit_enforcement() {
    let config = Arc::new(RateLimitConfig {
        enabled: true,
        websocket_limit: RateLimit {
            requests: 2,
            window: Duration::from_millis(100),
        }, // 2 req / 100ms
        method_limits: vec![],
        default_limit: RateLimit::default(),
    });
    let limiters = ManualRateLimiters::new(config).unwrap();
    let client1 = test_client_info(TEST_IP_1);

    // First two calls should succeed
    assert!(limiters.check_websocket_limit(&client1).is_ok());
    assert!(limiters.check_websocket_limit(&client1).is_ok());

    // Third call should fail (rate limited)
    let err = limiters.check_websocket_limit(&client1).unwrap_err();
    assert!(matches!(
        err,
        RateLimitError::ManualWebSocketLimitExceeded(_)
    ));

    // Wait for the window to pass
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Should succeed again after waiting
    assert!(limiters.check_websocket_limit(&client1).is_ok());
}

#[test]
fn test_check_websocket_limit_disabled() {
    let config = Arc::new(RateLimitConfig {
        enabled: false, // Rate limiting disabled
        websocket_limit: RateLimit {
            requests: 1,
            window: Duration::from_secs(1),
        }, // Strict limit
        method_limits: vec![],
        default_limit: RateLimit::default(),
    });
    let limiters = ManualRateLimiters::new(config).unwrap();
    let client1 = test_client_info(TEST_IP_1);

    // Should always succeed even with strict limit, because it's disabled
    assert!(limiters.check_websocket_limit(&client1).is_ok());
    assert!(limiters.check_websocket_limit(&client1).is_ok());
    assert!(limiters.check_websocket_limit(&client1).is_ok());
}

#[test]
fn test_check_websocket_limit_per_client() {
    let config = Arc::new(RateLimitConfig {
        enabled: true,
        websocket_limit: RateLimit {
            requests: 1,
            window: Duration::from_secs(60),
        }, // 1 req / min
        method_limits: vec![],
        default_limit: RateLimit::default(),
    });
    let limiters = ManualRateLimiters::new(config).unwrap();
    let client1 = test_client_info(TEST_IP_1);
    let client2 = test_client_info(TEST_IP_2);

    // Client 1 makes a request - succeeds
    assert!(limiters.check_websocket_limit(&client1).is_ok());

    // Client 1 makes another request - fails
    assert!(matches!(
        limiters.check_websocket_limit(&client1).unwrap_err(),
        RateLimitError::ManualWebSocketLimitExceeded(_)
    ));

    // Client 2 makes a request - succeeds (separate limit)
    assert!(limiters.check_websocket_limit(&client2).is_ok());

    // Client 2 makes another request - fails
    assert!(matches!(
        limiters.check_websocket_limit(&client2).unwrap_err(),
        RateLimitError::ManualWebSocketLimitExceeded(_)
    ));
}

// --- Tests for check_method_limit ---

#[tokio::test]
async fn test_check_method_limit_enforcement_and_pattern() {
    let config = Arc::new(RateLimitConfig {
        enabled: true,
        websocket_limit: RateLimit::default(), // Not used in this test
        method_limits: vec![
            MethodRateLimit {
                method_pattern: "get*".to_string(),
                limit: RateLimit {
                    requests: 2,
                    window: Duration::from_millis(100),
                }, // 2/100ms
            },
            MethodRateLimit {
                method_pattern: "transferTokens".to_string(),
                limit: RateLimit {
                    requests: 1,
                    window: Duration::from_millis(100),
                }, // 1/100ms
            },
        ],
        default_limit: RateLimit::default(),
    });
    let limiters = ManualRateLimiters::new(config).unwrap();
    let client1 = test_client_info(TEST_IP_1);

    // Test "get*" pattern
    assert!(limiters.check_method_limit(&client1, "getBalance").is_ok());
    assert!(limiters.check_method_limit(&client1, "getNonce").is_ok()); // Shares same limit
    let err_get = limiters
        .check_method_limit(&client1, "getBlock")
        .unwrap_err();
    assert!(matches!(
        err_get,
        RateLimitError::ManualMethodLimitExceeded(_)
    ));
    if let RateLimitError::ManualMethodLimitExceeded(msg) = err_get {
        assert!(msg.contains("get*")); // Check error message contains pattern
    }

    // Test specific "transferTokens" pattern
    assert!(limiters
        .check_method_limit(&client1, "transferTokens")
        .is_ok());
    let err_transfer = limiters
        .check_method_limit(&client1, "transferTokens")
        .unwrap_err();
    assert!(matches!(
        err_transfer,
        RateLimitError::ManualMethodLimitExceeded(_)
    ));
    if let RateLimitError::ManualMethodLimitExceeded(msg) = err_transfer {
        assert!(msg.contains("transferTokens"));
    }

    // Test non-matching method - should always succeed
    assert!(limiters
        .check_method_limit(&client1, "someOtherMethod")
        .is_ok());
    assert!(limiters
        .check_method_limit(&client1, "someOtherMethod")
        .is_ok());

    // Wait for window to pass
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Limits should reset
    assert!(limiters.check_method_limit(&client1, "getBalance").is_ok());
    assert!(limiters
        .check_method_limit(&client1, "transferTokens")
        .is_ok());
}

#[test]
fn test_check_method_limit_disabled() {
    let config = Arc::new(RateLimitConfig {
        enabled: false, // Disabled
        websocket_limit: RateLimit::default(),
        method_limits: vec![MethodRateLimit {
            method_pattern: "*".to_string(), // Match all
            limit: RateLimit {
                requests: 1,
                window: Duration::from_secs(1),
            }, // Strict limit
        }],
        default_limit: RateLimit::default(),
    });
    let limiters = ManualRateLimiters::new(config).unwrap();
    let client1 = test_client_info(TEST_IP_1);

    // Should always succeed because limiting is disabled
    assert!(limiters.check_method_limit(&client1, "anyMethod").is_ok());
    assert!(limiters
        .check_method_limit(&client1, "anotherMethod")
        .is_ok());
}

#[test]
fn test_check_method_limit_per_client_and_pattern() {
    let config = Arc::new(RateLimitConfig {
        enabled: true,
        websocket_limit: RateLimit::default(),
        method_limits: vec![
            MethodRateLimit {
                method_pattern: "get*".to_string(),
                limit: RateLimit {
                    requests: 1,
                    window: Duration::from_secs(60),
                }, // 1/min
            },
            MethodRateLimit {
                method_pattern: "transfer*".to_string(),
                limit: RateLimit {
                    requests: 1,
                    window: Duration::from_secs(60),
                }, // 1/min
            },
        ],
        default_limit: RateLimit::default(),
    });
    let limiters = ManualRateLimiters::new(config).unwrap();
    let client1 = test_client_info(TEST_IP_1);
    let client2 = test_client_info(TEST_IP_2);

    // Client 1 uses "get*"
    assert!(limiters.check_method_limit(&client1, "getBalance").is_ok());
    assert!(matches!(
        limiters
            .check_method_limit(&client1, "getNonce")
            .unwrap_err(),
        RateLimitError::ManualMethodLimitExceeded(_)
    ));

    // Client 1 uses "transfer*" - separate limit, should succeed
    assert!(limiters
        .check_method_limit(&client1, "transferTokens")
        .is_ok());
    assert!(matches!(
        limiters
            .check_method_limit(&client1, "transferFee")
            .unwrap_err(),
        RateLimitError::ManualMethodLimitExceeded(_)
    ));

    // Client 2 uses "get*" - separate client, should succeed
    assert!(limiters.check_method_limit(&client2, "getBalance").is_ok());
    assert!(matches!(
        limiters
            .check_method_limit(&client2, "getNonce")
            .unwrap_err(),
        RateLimitError::ManualMethodLimitExceeded(_)
    ));

    // Client 2 uses "transfer*" - separate client, should succeed
    assert!(limiters
        .check_method_limit(&client2, "transferTokens")
        .is_ok());
    assert!(matches!(
        limiters
            .check_method_limit(&client2, "transferFee")
            .unwrap_err(),
        RateLimitError::ManualMethodLimitExceeded(_)
    ));
}
