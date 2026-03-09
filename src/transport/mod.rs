#![cfg(feature = "mcp")]

use std::collections::HashMap;
use std::io::{BufRead, Write};

use serde_json::{json, Value};

use crate::model::{Command, ParsedCommand};
use crate::query::Registry;

/// An MCP (Model Context Protocol) server that exposes a [`Registry`]'s
/// commands as JSON-RPC 2.0 tools over a stdio transport.
///
/// The server reads newline-delimited JSON requests from a [`BufRead`] source
/// and writes newline-delimited JSON responses to a [`Write`] sink. Use
/// [`McpServer::serve_stdio`] to run against the process's stdin/stdout, or
/// [`McpServer::serve`] to inject any reader/writer pair (useful for tests).
///
/// Commands are exposed as MCP tools:
/// - Top-level commands → `"command-name"`
/// - Subcommands → `"parent-child"` (joined with `-`)
///
/// # Examples
///
/// ```no_run
/// # use argot_cmd::{Command, Registry};
/// # #[cfg(feature = "mcp")]
/// # {
/// use argot_cmd::McpServer;
///
/// let registry = Registry::new(vec![
///     Command::builder("ping").summary("Ping the server").build().unwrap(),
/// ]);
///
/// McpServer::new(registry)
///     .server_name("my-tool")
///     .server_version("1.0.0")
///     .serve_stdio()
///     .unwrap();
/// # }
/// ```
pub struct McpServer {
    registry: Registry,
    server_name: String,
    server_version: String,
}

impl McpServer {
    /// Create a new `McpServer` wrapping the given registry.
    ///
    /// The server name defaults to `"argot"` and version to `"0.1.0"`.
    /// Override with [`McpServer::server_name`] and [`McpServer::server_version`].
    pub fn new(registry: Registry) -> Self {
        Self {
            registry,
            server_name: "argot".to_string(),
            server_version: "0.1.0".to_string(),
        }
    }

    /// Set the server name returned in the `initialize` response.
    pub fn server_name(mut self, name: impl Into<String>) -> Self {
        self.server_name = name.into();
        self
    }

    /// Set the server version returned in the `initialize` response.
    pub fn server_version(mut self, version: impl Into<String>) -> Self {
        self.server_version = version.into();
        self
    }

    /// Run the MCP server, reading from `reader` and writing to `writer`.
    /// Blocks until EOF on reader.
    pub fn serve<R: BufRead, W: Write>(
        &self,
        mut reader: R,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                break; // EOF
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let request: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => {
                    let error = json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}});
                    writeln!(writer, "{}", error)?;
                    writer.flush()?;
                    continue;
                }
            };

            if let Some(response) = self.handle_request(&request) {
                writeln!(writer, "{}", response)?;
                writer.flush()?;
            }
        }
        Ok(())
    }

    /// Convenience: serve on stdin/stdout.
    pub fn serve_stdio(&self) -> std::io::Result<()> {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let reader = std::io::BufReader::new(stdin.lock());
        let mut writer = stdout.lock();
        self.serve(reader, &mut writer)
    }

    fn handle_request(&self, request: &Value) -> Option<Value> {
        // If no "id" field → notification → return None
        let id = request.get("id")?;

        let method = request.get("method")?.as_str().unwrap_or("");
        let params = request.get("params").unwrap_or(&Value::Null);

        Some(match method {
            "initialize" => self.handle_initialize(id),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(id, params),
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": "Method not found"
                }
            }),
        })
    }

    fn handle_initialize(&self, id: &Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": self.server_name,
                    "version": self.server_version
                }
            }
        })
    }

    fn handle_tools_list(&self, id: &Value) -> Value {
        let mut tools: Vec<Value> = Vec::new();
        for cmd in self.registry.list_commands() {
            Self::collect_tools(cmd, "", &mut tools);
        }
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": tools
            }
        })
    }

    fn collect_tools(cmd: &Command, prefix: &str, tools: &mut Vec<Value>) {
        tools.push(Self::command_to_tool(cmd, prefix));
        let sub_prefix = format!("{}{}-", prefix, cmd.canonical);
        for sub in &cmd.subcommands {
            Self::collect_tools(sub, &sub_prefix, tools);
        }
    }

    fn handle_tools_call(&self, id: &Value, params: &Value) -> Value {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Invalid params: missing 'name'"
                    }
                });
            }
        };

        let arguments = params.get("arguments").unwrap_or(&Value::Null);

        let cmd = match Self::find_command_by_tool_name(&self.registry, name) {
            Some(c) => c,
            None => {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": format!("unknown tool: {}", name)
                    }
                });
            }
        };

        // Build args HashMap from positional argument definitions + JSON values
        let mut args: HashMap<String, String> = HashMap::new();
        for arg_def in &cmd.arguments {
            if let Some(val) = arguments.get(&arg_def.name) {
                let val_str = value_to_string(val);
                args.insert(arg_def.name.clone(), val_str);
            } else if let Some(default) = &arg_def.default {
                args.insert(arg_def.name.clone(), default.clone());
            }
        }

        // Build flags HashMap from flag definitions + JSON values
        let mut flags: HashMap<String, String> = HashMap::new();
        for flag_def in &cmd.flags {
            if let Some(val) = arguments.get(&flag_def.name) {
                let val_str = value_to_string(val);
                flags.insert(flag_def.name.clone(), val_str);
            } else if let Some(default) = &flag_def.default {
                flags.insert(flag_def.name.clone(), default.clone());
            }
        }

        let parsed = ParsedCommand {
            command: cmd,
            args,
            flags,
        };

        match &cmd.handler {
            Some(handler) => match handler(&parsed) {
                Ok(()) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{"type": "text", "text": "Command executed successfully."}]
                    }
                }),
                Err(e) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{"type": "text", "text": format!("Error: {}", e)}],
                        "isError": true
                    }
                }),
            },
            None => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{"type": "text", "text": "Command has no handler."}]
                }
            }),
        }
    }

    fn command_to_tool(cmd: &Command, prefix: &str) -> Value {
        let tool_name = format!("{}{}", prefix, cmd.canonical);
        let desc = if !cmd.summary.is_empty() {
            &cmd.summary
        } else {
            &cmd.description
        };
        json!({
            "name": tool_name,
            "description": desc,
            "inputSchema": Self::build_input_schema(cmd)
        })
    }

    fn build_input_schema(cmd: &Command) -> Value {
        let mut properties: serde_json::Map<String, Value> = serde_json::Map::new();
        let mut required: Vec<String> = Vec::new();

        // Positional arguments → string properties
        for arg in &cmd.arguments {
            properties.insert(
                arg.name.clone(),
                json!({"type": "string", "description": arg.description}),
            );
            if arg.required {
                required.push(arg.name.clone());
            }
        }

        // Flags → string or boolean properties
        for flag in &cmd.flags {
            let prop_type = if flag.takes_value {
                "string"
            } else {
                "boolean"
            };
            properties.insert(
                flag.name.clone(),
                json!({"type": prop_type, "description": flag.description}),
            );
            if flag.required {
                required.push(flag.name.clone());
            }
        }

        let mut schema = json!({
            "type": "object",
            "properties": properties
        });

        if !required.is_empty() {
            schema["required"] = json!(required);
        }

        schema
    }

    /// Find a command by its MCP tool name (e.g. "parent-child").
    /// Tool names for top-level commands are just their canonical name.
    /// Subcommands use "parent-child" (joined with "-").
    fn find_command_by_tool_name<'a>(registry: &'a Registry, name: &str) -> Option<&'a Command> {
        for cmd in registry.list_commands() {
            if let Some(found) = find_in_tree(cmd, "", name) {
                return Some(found);
            }
        }
        None
    }
}

/// Recursively search for a command matching the given tool name.
fn find_in_tree<'a>(cmd: &'a Command, prefix: &str, target: &str) -> Option<&'a Command> {
    let tool_name = format!("{}{}", prefix, cmd.canonical);
    let sub_prefix = format!("{}-", tool_name);

    if tool_name == target {
        return Some(cmd);
    }

    // Only descend into subcommands if target could match a deeper path
    if target.starts_with(&sub_prefix) {
        for sub in &cmd.subcommands {
            if let Some(found) = find_in_tree(sub, &sub_prefix, target) {
                return Some(found);
            }
        }
    }

    None
}

/// Convert a JSON value to a string representation suitable for CLI-style parsing.
fn value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::Arc;

    fn make_registry() -> Registry {
        let deploy = Command::builder("deploy")
            .summary("Deploy the application")
            .argument(
                crate::model::Argument::builder("env")
                    .description("target environment")
                    .required()
                    .build()
                    .unwrap(),
            )
            .flag(
                crate::model::Flag::builder("dry-run")
                    .description("dry run mode")
                    .build()
                    .unwrap(),
            )
            .handler(Arc::new(|_parsed| Ok(())))
            .build()
            .unwrap();

        let sub_cmd = Command::builder("rollback")
            .summary("Rollback the deployment")
            .build()
            .unwrap();

        let deploy_with_sub = Command::builder("service")
            .summary("Service management")
            .subcommand(sub_cmd)
            .build()
            .unwrap();

        Registry::new(vec![deploy, deploy_with_sub])
    }

    fn run_server(input: &str) -> String {
        let registry = make_registry();
        let server = McpServer::new(registry);
        run_server_with(&server, input)
    }

    fn run_server_with(server: &McpServer, input: &str) -> String {
        let reader = Cursor::new(input.as_bytes().to_vec());
        let mut output = Vec::new();
        server.serve(reader, &mut output).unwrap();
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn test_tools_list() {
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#,
            "\n"
        );
        let output = run_server(input);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);

        // Check initialize response
        let init_resp: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(init_resp["id"], 0);
        assert!(init_resp["result"]["capabilities"].is_object());

        // Check tools/list response
        let list_resp: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(list_resp["id"], 1);
        let tools = list_resp["result"]["tools"].as_array().unwrap();
        let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(
            tool_names.contains(&"deploy"),
            "expected 'deploy' in tools: {:?}",
            tool_names
        );
        assert!(
            tool_names.contains(&"service"),
            "expected 'service' in tools: {:?}",
            tool_names
        );
        assert!(
            tool_names.contains(&"service-rollback"),
            "expected 'service-rollback' in tools: {:?}",
            tool_names
        );
    }

    #[test]
    fn test_tools_call_with_handler() {
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"deploy","arguments":{"env":"prod","dry-run":true}}}"#,
            "\n"
        );
        let output = run_server(input);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1);

        let resp: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(resp["id"], 2);
        assert!(
            resp["error"].is_null(),
            "expected no error, got: {}",
            resp["error"]
        );
        let content = resp["result"]["content"].as_array().unwrap();
        assert!(!content.is_empty());
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Command executed successfully.");
    }

    #[test]
    fn test_tools_call_unknown_tool() {
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nonexistent","arguments":{}}}"#,
            "\n"
        );
        let output = run_server(input);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1);

        let resp: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(resp["id"], 3);
        assert!(!resp["error"].is_null(), "expected error for unknown tool");
        assert_eq!(resp["error"]["code"], -32602);
        let msg = resp["error"]["message"].as_str().unwrap();
        assert!(
            msg.contains("nonexistent"),
            "error message should mention tool name: {}",
            msg
        );
    }

    #[test]
    fn test_notification_no_response() {
        // "initialized" has no "id" field → notification → no response
        let input = concat!(
            r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
            "\n"
        );
        let output = run_server(input);
        assert!(
            output.trim().is_empty(),
            "expected no output for notification, got: {:?}",
            output
        );
    }

    #[test]
    fn test_invalid_json() {
        let input = "this is not json\n";
        let output = run_server(input);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1);

        let resp: Value = serde_json::from_str(lines[0]).unwrap();
        assert!(!resp["error"].is_null(), "expected parse error response");
        assert_eq!(resp["error"]["code"], -32700);
        assert_eq!(resp["error"]["message"], "Parse error");
    }

    #[test]
    fn test_tools_call_no_handler() {
        // "service" command has no handler
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"service","arguments":{}}}"#,
            "\n"
        );
        let output = run_server(input);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1);

        let resp: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(resp["id"], 4);
        assert!(
            resp["error"].is_null(),
            "expected no JSON-RPC error, got: {}",
            resp["error"]
        );
        let content = resp["result"]["content"].as_array().unwrap();
        assert_eq!(content[0]["text"], "Command has no handler.");
    }

    #[test]
    fn test_method_not_found() {
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":5,"method":"unknown/method","params":{}}"#,
            "\n"
        );
        let output = run_server(input);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1);

        let resp: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(resp["id"], 5);
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn test_tools_list_three_level_nesting() {
        use serde_json::Value;
        // Build a 3-level tree: service → deployment → blue-green
        let leaf = crate::model::Command::builder("blue-green")
            .summary("Blue-green deployment strategy")
            .build()
            .unwrap();
        let mid = crate::model::Command::builder("deployment")
            .summary("Deployment operations")
            .subcommand(leaf)
            .build()
            .unwrap();
        let top = crate::model::Command::builder("service")
            .summary("Service management")
            .subcommand(mid)
            .build()
            .unwrap();
        let registry = crate::query::Registry::new(vec![top]);
        let server = McpServer::new(registry);

        let input = concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#,
            "\n"
        );
        let output = run_server_with(&server, input);
        let line = output.lines().next().unwrap();
        let resp: Value = serde_json::from_str(line).unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

        assert!(tool_names.contains(&"service"), "top-level");
        assert!(tool_names.contains(&"service-deployment"), "2nd level");
        assert!(
            tool_names.contains(&"service-deployment-blue-green"),
            "3rd level"
        );
    }

    #[test]
    fn test_build_input_schema() {
        let cmd = Command::builder("deploy")
            .argument(
                crate::model::Argument::builder("env")
                    .description("target environment")
                    .required()
                    .build()
                    .unwrap(),
            )
            .flag(
                crate::model::Flag::builder("dry-run")
                    .description("dry run mode")
                    .build()
                    .unwrap(),
            )
            .flag(
                crate::model::Flag::builder("output")
                    .description("output format")
                    .takes_value()
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let schema = McpServer::build_input_schema(&cmd);
        assert_eq!(schema["type"], "object");

        let props = &schema["properties"];
        assert_eq!(props["env"]["type"], "string");
        assert_eq!(props["dry-run"]["type"], "boolean");
        assert_eq!(props["output"]["type"], "string");

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("env")));
        assert!(!required.contains(&json!("dry-run")));
    }
}
