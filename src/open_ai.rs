use std::fmt;

use anyhow::{bail, Result};
use async_openai::{
    config::OpenAIConfig,
    types::{
        AssistantToolFileSearchResources, AssistantToolsFileSearch, CreateAssistantRequestArgs,
        CreateFileRequest, CreateMessageRequest, CreateRunRequestArgs, CreateThreadRequest,
        CreateVectorStoreRequest, FilePurpose, MessageContent, MessageRole, ModifyAssistantRequest,
        RunStatus,
    },
    Client,
};

#[derive(Debug)]
pub struct ChetGPTWrapper {
    client: Client<OpenAIConfig>,
    thread_id: String,
    assistant_id: String,
}

#[derive(Debug, Clone)]
struct GPTCallError;

impl fmt::Display for GPTCallError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error calling gpt for ChetGPTWrapper")
    }
}

static PROMPT: &str = "You are Chet, a 24 year old computer programmer and video game fan talking with \
his friends in discord. The attached file describes your personality in more detail. Use that file to \
respond in character to the discord messages being sent to you. The information in the file is facts \
about you. Rather than simply repeating these facts, try to act like Chet would without contradicting \
the provided facts. Speak less enthusiastically and more ironically and cynically than ChatGPT normally would.";

// This wrapper creates an assistant that reads a file "../files/personality.txt" and responds as if it
// is a person named "Chet" who has the personality described by that file
impl ChetGPTWrapper {
    pub async fn new() -> Self {
        // instantiate client
        let client = Client::new();

        // create assistant request
        let create_assistant_request = CreateAssistantRequestArgs::default()
            .name("Chet")
            .instructions(&PROMPT.to_string())
            .model("gpt-4o")
            .tools(vec![AssistantToolsFileSearch::default().into()])
            .build()
            .expect("failed to create assistant");

        // create assistant object
        let assistant = client
            .assistants()
            .create(create_assistant_request)
            .await
            .expect("error creating assistant");

        // upload file to add to vector store
        let openai_file = client
            .files()
            .create(CreateFileRequest {
                file: "./files/personality.txt".into(),
                purpose: FilePurpose::Assistants,
            })
            .await
            .expect("failed to upload file");

        // Create a vector store called "Chet Personality Docs"
        // add uploaded file to vector store
        let vector_store = client
            .vector_stores()
            .create(CreateVectorStoreRequest {
                name: Some("Chet Personality Docs".into()),
                file_ids: Some(vec![openai_file.id.clone()]),
                ..Default::default()
            })
            .await
            .expect("failed to create vector store");

        // update assistant to use file/vector store
        let assistant = client
            .assistants()
            .update(
                &assistant.id,
                ModifyAssistantRequest {
                    tool_resources: Some(
                        AssistantToolFileSearchResources {
                            vector_store_ids: vec![vector_store.id.clone()],
                        }
                        .into(),
                    ),
                    ..Default::default()
                },
            )
            .await
            .expect("failed to update assistant to use vector store");

        // Create a thread
        let thread = client
            .threads()
            .create(CreateThreadRequest::default())
            .await
            .expect("couldn't create thread");

        Self {
            client: client,
            thread_id: thread.id,
            assistant_id: assistant.id,
        }
    }

    // Say something to Chet and get a response
    // By using threads, gpt will continue with conversation where it left off each time this method is called
    pub async fn get_gpt_response(&self, input_str: &str) -> Result<String> {
        // create a message
        let _message = self
            .client
            .threads()
            .messages(&self.thread_id)
            .create(CreateMessageRequest {
                role: MessageRole::User,
                content: input_str.into(),
                ..Default::default()
            })
            .await?;

        //create a run for the thread
        let run_request = CreateRunRequestArgs::default()
            .assistant_id(&self.assistant_id)
            .parallel_tool_calls(false)
            .build()?;
        let run = self
            .client
            .threads()
            .runs(&self.thread_id)
            .create(run_request)
            .await
            .expect("failed to create run");

        //wait for the run to complete
        loop {
            //retrieve the run
            let run = self
                .client
                .threads()
                .runs(&self.thread_id)
                .retrieve(&run.id)
                .await?;
            //check the status of the run
            match run.status {
                RunStatus::Completed => {
                    //retrieve the response from the run
                    let response = self
                        .client
                        .threads()
                        .messages(&self.thread_id)
                        .list(&[("limit", "10")])
                        .await?;
                    //get the message id from the response
                    let message_id = response.data.first().unwrap().id.clone();
                    //get the message from the response
                    let message = self
                        .client
                        .threads()
                        .messages(&self.thread_id)
                        .retrieve(&message_id)
                        .await?;
                    //get the content from the message
                    let content = message.content.first().unwrap();
                    //get the text from the content
                    let text = match content {
                        MessageContent::Text(text) => text.text.value.clone(),
                        MessageContent::ImageFile(_) | MessageContent::ImageUrl(_) => {
                            panic!("imaged are not expected in this example");
                        }
                    };
                    //print the text
                    return Ok(text);
                }
                RunStatus::Failed => {
                    println!("> Run Failed: {:#?}", run);
                    bail!(GPTCallError);
                }
                RunStatus::Queued => {
                    println!("> Run Queued");
                }
                RunStatus::Cancelling => {
                    println!("> Run Cancelling");
                }
                RunStatus::Cancelled => {
                    println!("> Run Cancelled");
                    break;
                }
                RunStatus::Expired => {
                    println!("> Run Expired");
                    break;
                }
                RunStatus::RequiresAction => {
                    println!("> Run Requires Action");
                }
                RunStatus::InProgress => {
                    println!("> In Progress ...");
                }
                RunStatus::Incomplete => {
                    println!("> Run Incomplete");
                }
            }
            //wait for 1 second before checking the status again
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        panic!("No response from GPT when conducting run");
    }
}
