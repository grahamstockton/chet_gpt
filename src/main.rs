mod commands;
mod open_ai;

use anyhow::{Context as _, Error};
use open_ai::ChetGPTWrapper;
use poise::serenity_prelude as serenity;
use shuttle_runtime::{SecretStore, Secrets};
use shuttle_serenity::ShuttleSerenity;

type Context<'a> = poise::Context<'a, ChetGPTWrapper, Error>;

/**
* Runtime for our program. Runs on poise, a runtime framework for creating discord bots. Deploys
* using shuttle.
*/
#[shuttle_runtime::main]
async fn poise(#[Secrets] secret_store: SecretStore) -> ShuttleSerenity {

    // get discord token and api key from secret store
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;
    let api_key = secret_store
        .get("OPEN_AI_TOKEN")
        .context("'OPEN_AI_TOKEN' was not found")?;

    // set env variable for api
    std::env::set_var("OPENAI_API_KEY", api_key);
    // this method needs to be inside main() method
    std::env::set_var("RUST_BACKTRACE", "1");

    // dial up gpt client
    let gpt_client = ChetGPTWrapper::new().await;

    // initialize poise framework
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::chet_gpt()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {

                // register commands
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                // register slash commands
                let create_commands =
                    poise::builtins::create_application_commands(&framework.options().commands);
                serenity::Command::set_global_commands(ctx, create_commands).await?;

                Ok(gpt_client)
            })
        })
        .build();

    // build client
    let client =
        serenity::ClientBuilder::new(discord_token, serenity::GatewayIntents::non_privileged())
            .framework(framework)
            .await
            .map_err(shuttle_runtime::CustomError::new)?;

    Ok(client.into())
}
