use std::fmt::{Debug, Display};
use tokio::task::JoinError;
use zero2prod::{
    configuration::get_configuration,
    idempotency,
    issue_delivery_worker::run_worker_until_stopped,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let configuration = get_configuration().expect("Failed to read configuration.");

    let application = Application::build(configuration.clone()).await?;
    let application_task = tokio::spawn(application.run_until_stopped());
    let worker_task = tokio::spawn(run_worker_until_stopped(configuration.clone()));
    let idempotency_keys_remover_task = tokio::spawn(idempotency::run_worker_until_stopped(
        configuration.database,
    ));
    tokio::select! {
        o = application_task => report_exit("API", o),
        o = worker_task => report_exit("Newsletter email delivery worker", o),
        o = idempotency_keys_remover_task => report_exit("Idempotency keys remover worker", o)
    };

    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name)
        }
        Ok(Err(e)) => {
            tracing::error!(
            error.cause_chain = ?e,
            error.message = %e,
            "{} failed",
            task_name
            )
        }
        Err(e) => {
            tracing::error!(
            error.cause_chain = ?e,
            error.message = %e,
            "{}' task failed to complete",
            task_name
            )
        }
    }
}
