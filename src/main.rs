use color_eyre::eyre::eyre;
use color_eyre::Result;
use gadget_sdk::info;
use gadget_sdk::job_runner::{JobBuilder, MultiJobRunner};
use gadget_sdk::subxt_core::tx::signer::Signer;
use tangle_avs as blueprint;
use tangle_avs::tangle_avs_registration;

#[gadget_sdk::main(env)]
async fn main() {
    let client = env.client().await.map_err(|e| eyre!(e))?;
    let signer = env.first_sr25519_signer().map_err(|e| eyre!(e))?;

    info!("Starting the event watcher for {} ...", signer.account_id());

    let context = blueprint::BalanceTransferContext {
        client: client.clone(),
        address: Default::default(),
    };

    let tangle_avs = blueprint::RegisterToTangleEventHandler {
        service_id: env.service_id.unwrap(),
        context: context.clone(),
        client,
        signer,
    };

    info!("~~~ Executing the Tangle AVS ~~~");
    MultiJobRunner::new(env)
        .job(JobBuilder::new(tangle_avs).registration(context, tangle_avs_registration))
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
// #[tokio::main]
// async fn main() -> Result<()> {
//     gadget_sdk::logging::setup_log();
//     println!("{}", tangle_avs::utils::tangle::TANGLE_AVS_ASCII);
//     // Load the environment and create the gadget runner
//     let config = ContextConfig::from_args();
//     let env = gadget_sdk::config::load(config).expect("Failed to load environment");
//     let mut runner = Box::new(TangleGadgetRunner { env: env.clone() });
//
//     info!("~~~ Executing the Tangle AVS ~~~");
//
//     info!("Registering...");
//     // Register the operator if needed
//     if env.should_run_registration() {
//         runner.register().await?;
//     }
//
//     info!("Running...");
//     // Run the gadget / AVS
//     runner.run().await?;
//
//     info!("Exiting...");
//     Ok(())
// }
