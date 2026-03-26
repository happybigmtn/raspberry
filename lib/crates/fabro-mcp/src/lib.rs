pub mod client;
mod client_handler;
pub mod config;
pub mod connection_manager;

#[cfg(test)]
mod tests {
    use crate::config::{McpServerConfig, McpTransport};
    use std::path::PathBuf;

    #[test]
    fn mcp_transport_stdio_command_parsing() {
        let config = McpServerConfig {
            name: "test-server".to_string(),
            transport: McpTransport::Stdio {
                command: vec!["npx".to_string(), "-y".to_string(), "mcp-server".to_string()],
                env: std::collections::HashMap::new(),
            },
            startup_timeout_secs: 30,
            tool_timeout_secs: 60,
        };

        match &config.transport {
            McpTransport::Stdio { command, env } => {
                assert_eq!(command.len(), 3);
                assert_eq!(command[0], "npx");
                assert!(env.is_empty());
            }
            _ => panic!("expected Stdio transport"),
        }
    }

    #[test]
    fn mcp_transport_stdio_with_env() {
        let mut env = std::collections::HashMap::new();
        env.insert("API_KEY".to_string(), "secret".to_string());

        let config = McpServerConfig {
            name: "env-server".to_string(),
            transport: McpTransport::Stdio {
                command: vec!["node".to_string(), "server.js".to_string()],
                env,
            },
            startup_timeout_secs: 30,
            tool_timeout_secs: 60,
        };

        match &config.transport {
            McpTransport::Stdio { command, env } => {
                assert_eq!(env.get("API_KEY"), Some(&"secret".to_string()));
            }
            _ => panic!("expected Stdio transport"),
        }
    }

    #[test]
    fn mcp_transport_http_url_parsing() {
        let config = McpServerConfig {
            name: "http-server".to_string(),
            transport: McpTransport::Http {
                url: "http://localhost:8080/mcp".to_string(),
                headers: std::collections::HashMap::new(),
            },
            startup_timeout_secs: 30,
            tool_timeout_secs: 60,
        };

        match &config.transport {
            McpTransport::Http { url, headers } => {
                assert!(url.starts_with("http://localhost"));
                assert!(headers.is_empty());
            }
            _ => panic!("expected Http transport"),
        }
    }

    #[test]
    fn mcp_transport_http_with_headers() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());

        let config = McpServerConfig {
            name: "auth-server".to_string(),
            transport: McpTransport::Http {
                url: "http://localhost:8080/mcp".to_string(),
                headers,
            },
            startup_timeout_secs: 30,
            tool_timeout_secs: 60,
        };

        match &config.transport {
            McpTransport::Http { headers, .. } => {
                assert_eq!(
                    headers.get("Authorization"),
                    Some(&"Bearer token123".to_string())
                );
            }
            _ => panic!("expected Http transport"),
        }
    }

    #[test]
    fn server_config_default_timeouts() {
        let config = McpServerConfig {
            name: "test".to_string(),
            transport: McpTransport::Http {
                url: "http://localhost:8080".to_string(),
                headers: std::collections::HashMap::new(),
            },
            startup_timeout_secs: 30,
            tool_timeout_secs: 60,
        };

        assert_eq!(config.startup_timeout_secs, 30);
        assert_eq!(config.tool_timeout_secs, 60);
    }
}
