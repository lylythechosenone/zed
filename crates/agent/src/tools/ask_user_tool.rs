use anyhow::Result;
use gpui::{App, SharedString, Task};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{AgentTool, ToolCallEventStream, thread::ToolInput};
use agent_client_protocol as acp;
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct AskUserArgs {
    /// The prompt to display to the user.
    pub prompt: String,
}

pub struct AskUserTool;

impl AgentTool for AskUserTool {
    type Input = AskUserArgs;
    type Output = String;

    const NAME: &'static str = "ask_user";

    fn description() -> SharedString {
        "Ask the user a question and wait for their free-text input.".into()
    }

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Fetch
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        input
            .map(|x| x.prompt)
            .unwrap_or_else(|_| "Asking a question...".to_string())
            .into()
    }

    fn run(
        self: Arc<Self>,
        _input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, String>> {
        cx.spawn(async move |_cx| {
            // Wait for the UI to submit the response
            let user_response = event_stream
                .request_input()
                .await
                .map_err(|e| format!("Input request cancelled: {}", e))?;

            Ok(user_response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_ask_user_tool(cx: &mut TestAppContext) {
        let tool = Arc::new(AskUserTool);
        let (event_stream, mut event_rx) = ToolCallEventStream::test();

        let input = AskUserArgs {
            prompt: "How are you?".to_string(),
        };

        let task = cx.update(|cx| tool.run(ToolInput::resolved(input), event_stream, cx));

        // Expect the fields update (content)
        let update_fields = event_rx.expect_update_fields().await;
        assert_eq!(update_fields.content.unwrap().len(), 1);

        // Expect the input request
        let req = event_rx.expect_input_request().await;

        // Respond to the input request
        req.response.send("I am doing great!".to_string()).unwrap();

        // Wait for the tool to finish
        let result = task.await.unwrap();
        assert_eq!(result, "I am doing great!");
    }

    #[gpui::test]
    async fn test_ask_user_tool_cancellation(cx: &mut TestAppContext) {
        let tool = Arc::new(AskUserTool);
        let (event_stream, mut event_rx) = ToolCallEventStream::test();

        let input = AskUserArgs {
            prompt: "How are you?".to_string(),
        };

        let task = cx.update(|cx| tool.run(ToolInput::resolved(input), event_stream, cx));

        // Expect the fields update (content)
        let _ = event_rx.expect_update_fields().await;

        // Expect the input request
        let req = event_rx.expect_input_request().await;

        // Cancel by dropping the response channel
        drop(req.response);

        // Wait for the tool to finish
        let err = task.await.unwrap_err();
        assert!(err.contains("Input request cancelled"));
    }
}
