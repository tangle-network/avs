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
