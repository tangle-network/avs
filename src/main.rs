use color_eyre::eyre::eyre;
use color_eyre::Result;
use gadget_sdk::info;
use gadget_sdk::runners::eigenlayer::EigenlayerECDSAConfig;
use gadget_sdk::runners::BlueprintRunner;
use gadget_sdk::subxt_core::tx::signer::Signer;
use tangle_avs as blueprint;
use tangle_avs::RegisterToTangleEventHandler;

#[gadget_sdk::main(env)]
async fn main() {
    let client = env.client().await.map_err(|e| eyre!(e))?;
    let signer = env.first_sr25519_signer().map_err(|e| eyre!(e))?;

    info!("Starting the event watcher for {} ...", signer.account_id());

    let context = blueprint::BalanceTransferContext {
        client: client.clone(),
        env: env.clone(),
    };

    let tangle_avs = RegisterToTangleEventHandler {
        service_id: 0,
        context: context.clone(),
        client,
        signer,
    };

    info!("~~~ Executing the Tangle AVS ~~~");
    let eigen_config = EigenlayerECDSAConfig {};
    BlueprintRunner::new(eigen_config, env.clone())
        .job(tangle_avs)
        .run()
        .await
        .unwrap();

    info!("Exiting...");
    Ok(())
}
