use color_eyre::eyre::eyre;
use color_eyre::Result;
use gadget_sdk::config::protocol::TangleInstanceSettings;
use gadget_sdk::info;
use gadget_sdk::runners::tangle::TangleConfig;
use gadget_sdk::runners::BlueprintRunner;
use gadget_sdk::subxt_core::tx::signer::Signer;
use tangle_avs as blueprint;
use tangle_avs::{tangle_avs_registration, RegisterToTangleEventHandler};

#[gadget_sdk::main(env)]
async fn main() {
    let client = env.client().await.map_err(|e| eyre!(e))?;
    let signer = env.first_sr25519_signer().map_err(|e| eyre!(e))?;

    info!("Starting the event watcher for {} ...", signer.account_id());

    let context = blueprint::BalanceTransferContext {
        client: client.clone(),
        address: Default::default(),
    };

    tangle_avs_registration(&env.clone(), context.clone())
        .await
        .unwrap();

    let tangle_settings = env.protocol_specific.tangle().unwrap();
    let TangleInstanceSettings { service_id, .. } = tangle_settings;

    let tangle_avs = RegisterToTangleEventHandler {
        service_id: *service_id,
        context: context.clone(),
        client,
        signer,
    };

    info!("~~~ Executing the Tangle AVS ~~~");
    let tangle_config = TangleConfig {
        price_targets: Default::default(),
    };
    BlueprintRunner::new(tangle_config, env.clone())
        .job(tangle_avs)
        .run()
        .await
        .unwrap();

    info!("Exiting...");
    Ok(())
}
