use serde_json::json;
use windows::Win32::{
    Foundation::CloseHandle,
    System::{
        Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory},
        Threading::{OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE},
    },
};
mod hack;

use rmcp::{
    Error as McpError, RoleServer, ServerHandler, const_string, model::*, schemars,
    service::RequestContext, tool,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StructRequest {
    pub request_amount: i32,
}

#[derive(Debug, Clone)]
pub struct PersonaMoney;

#[tool(tool_box)]
impl PersonaMoney {
    pub fn new() -> Self {
        Self
    }

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    #[tool(description = "Set in-game money")]
    async fn set_money(
        &self,
        #[tool(aggr)] StructRequest { request_amount }: StructRequest,
    ) -> Result<CallToolResult, McpError> {
        let process_name = "P4G.exe";
        let money_value = request_amount;

        let process_id = hack::find_process_id(process_name);

        if process_id.is_none() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Process {} not found",
                process_name
            ))]));
        }

        let pid = process_id.unwrap();

        unsafe {
            let handle = OpenProcess(
                PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_VM_READ,
                false,
                pid,
            );

            if handle.is_err() {
                return Ok(CallToolResult::error(vec![Content::text(
                    "❌ Failed to open process".to_string(),
                )]));
            }

            let handle = handle.unwrap();

            let base_address = hack::get_base_address(pid).unwrap_or(0);
            tracing::debug!("base_address = {:X}", base_address);

            if base_address == 0 {
                return Ok(CallToolResult::error(vec![Content::text(
                    "❌ Failed to get base address".to_string(),
                )]));
            }

            let target_address = (base_address + 0x1165900) as *mut core::ffi::c_void;
            tracing::debug!("target_address = {:X}", target_address as usize);

            // step 1: get the pointer address
            let pointer_address = (base_address + 0x1165900) as *const core::ffi::c_void;

            // step 2: read the pointer value (which points to the real money address)
            let mut money_address: usize = 0;
            let read_result = ReadProcessMemory(
                handle,
                pointer_address,
                &mut money_address as *mut _ as *mut core::ffi::c_void,
                std::mem::size_of::<usize>(),
                None,
            );

            if read_result.is_err() || money_address == 0 {
                return Ok(CallToolResult::error(vec![Content::text(
                    "❌ Failed to read memory".to_string(),
                )]));
            }

            // step 3: write to the final address
            let money_ptr = money_address as *mut core::ffi::c_void;
            let result = WriteProcessMemory(
                handle,
                money_ptr,
                &money_value as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<i32>(),
                None,
            );

            if result.is_err() {
                return Ok(CallToolResult::error(vec![Content::text(
                    "❌ Failed to write memory".to_string(),
                )]));
            }

            CloseHandle(handle).unwrap();
            tracing::debug!("Successfully to close handle");

            Ok(CallToolResult::success(vec![Content::text(format!(
                "✅ Successfully set money to {}",
                money_value
            ))]))
        }
    }
}

const_string!(Echo = "echo");
#[tool(tool_box)]
impl ServerHandler for PersonaMoney {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server is provide real-time money hack within 'set_money'".to_string(),
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                self._create_resource_text("str:////Users/to/some/path/", "cwd"),
                self._create_resource_text("memo://insights", "memo-name"),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match uri.as_str() {
            "str:////Users/to/some/path/" => {
                let cwd = "/Users/to/some/path/";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(cwd, uri)],
                })
            }
            "memo://insights" => {
                let memo = "Business Intelligence Memo\n\nAnalysis has revealed 5 key insights ...";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(memo, uri)],
                })
            }
            _ => Err(McpError::resource_not_found(
                "resource_not_found",
                Some(json!({
                    "uri": uri
                })),
            )),
        }
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![Prompt::new(
                "example_prompt",
                Some("This is an example prompt that takes one required argument, message"),
                Some(vec![PromptArgument {
                    name: "message".to_string(),
                    description: Some("A message to put in the prompt".to_string()),
                    required: Some(true),
                }]),
            )],
        })
    }

    async fn get_prompt(
        &self,
        GetPromptRequestParam { name, arguments }: GetPromptRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        match name.as_str() {
            "example_prompt" => {
                let message = arguments
                    .and_then(|json| json.get("message")?.as_str().map(|s| s.to_string()))
                    .ok_or_else(|| {
                        McpError::invalid_params("No message provided to example_prompt", None)
                    })?;

                let prompt =
                    format!("This is an example prompt with your message here: '{message}'");
                Ok(GetPromptResult {
                    description: None,
                    messages: vec![PromptMessage {
                        role: PromptMessageRole::User,
                        content: PromptMessageContent::text(prompt),
                    }],
                })
            }
            _ => Err(McpError::invalid_params("prompt not found", None)),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: PaginatedRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }
}
