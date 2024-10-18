use color_eyre::Result;
use gadget_sdk::{config::ContextConfig, info, run::GadgetRunner};
use structopt::StructOpt;
use tangle_avs::TangleGadgetRunner;

#[tokio::main]
async fn main() -> Result<()> {
    gadget_sdk::logging::setup_log();
    println!("{}", tangle_avs::utils::tangle::TANGLE_AVS_ASCII);
    // Load the environment and create the gadget runner
    let config = ContextConfig::from_args();
    let env = gadget_sdk::config::load(config).expect("Failed to load environment");
    let mut runner = Box::new(TangleGadgetRunner { env: env.clone() });

    info!("~~~ Executing the Tangle AVS ~~~");

    info!("Registering...");
    // Register the operator if needed
    if env.should_run_registration() {
        runner.register().await?;
    }

    info!("Running...");
    // Run the gadget / AVS
    runner.run().await?;

    info!("Exiting...");
    Ok(())
}
