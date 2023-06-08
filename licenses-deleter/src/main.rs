/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

#![feature(future_join)]

use lambda_runtime::{Error, LambdaEvent};
use licenses_core::{run_lambda, LicenseDao};
use serde::Deserialize;
use tokio::main as tokio_main;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    customer_id: Uuid,
    vessel_id: Uuid,
    license_key: String,
}

#[tokio_main]
async fn main() -> Result<(), Error> {
    let dao = &LicenseDao::load_from_env().await?;

    run_lambda!(move |event: LambdaEvent<Request>| async move {
        dao.delete_license(
            event.payload.customer_id,
            event.payload.vessel_id,
            event.payload.license_key,
        )
        .await
    })
}

#[cfg(test)]
mod tests {
    use crate::Request;
    use serde_json::from_str;
    use uuid::{uuid, Uuid};

    const CUSTOMER_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000000");
    const VESSEL_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
    const LICENSE_KEY: &str = "WEATHER_FORECAST";

    #[test]
    fn deserialize_request() {
        let input = format!(
            "{{\"customerId\":\"{CUSTOMER_ID}\",\"vesselId\":\"{VESSEL_ID}\",\"licenseKey\":\"{LICENSE_KEY}\"}}"
        );
        let request: Request = from_str(&input).unwrap();

        assert_eq!(CUSTOMER_ID, request.customer_id);
        assert_eq!(VESSEL_ID, request.vessel_id);
        assert_eq!(LICENSE_KEY, request.license_key);
    }
}
