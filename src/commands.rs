use crate::Context;
use anyhow::{Error, Result};

/// Poise command to call ChetGPT and get a response.
/// This call is stateful, and will remember previous calls.
/// Currently, state is only remember for the duration the bot is running.
#[poise::command(slash_command)]
pub async fn chet_gpt(
    ctx: Context<'_>,
    #[description = "say something to chetGPT"] message: String,
) -> Result<(), Error> {
    ctx.defer().await?; // this keeps us from timing out
    let response = ctx
        .data()
        .get_gpt_response(&message)
        .await
        .expect("failed to call gpt");
    ctx.say(response).await?;

    Ok(())
}
