/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

#![feature(future_join)]

use chrono::{DateTime, FixedOffset};
use lambda_runtime::{Error, LambdaEvent};
use licenses_core::{run_lambda, ApiError, License, LicenseDao};
use serde::Deserialize;
use tokio::main as tokio_main;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    customer_id: Uuid,
    vessel_id: Uuid,
    license_key: String,
    count: Option<u8>,
    expires_at: Option<DateTime<FixedOffset>>,
}

#[tokio_main]
async fn main() -> Result<(), Error> {
    let dao = &LicenseDao::load_from_env().await?;

    run_lambda!(move |event: LambdaEvent<Request>| async move {
        dao.create_license(License {
            customer_id: event.payload.customer_id,
            vessel_id: event.payload.vessel_id,
            license_key: event.payload.license_key.clone(),
            count: event.payload.count,
            expires_at: event.payload.expires_at,
        })
        .await?;

        Ok::<String, ApiError>(event.payload.license_key)
    })
}

#[cfg(test)]
mod tests {
    use crate::Request;
    use chrono::{DateTime, FixedOffset};
    use serde_json::from_str;
    use uuid::{uuid, Uuid};

    const CUSTOMER_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000000");
    const VESSEL_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
    const LICENSE_KEY: &str = "weather0";
    const COUNT: u8 = 2;

    #[test]
    fn deserialize_request() {
        let input = format!(
            "{{\"customerId\":\"{CUSTOMER_ID}\",\"vesselId\":\"{VESSEL_ID}\",\"licenseKey\":\"{LICENSE_KEY}\"}}"
        );
        let request: Request = from_str(&input).unwrap();

        assert_eq!(CUSTOMER_ID, request.customer_id);
        assert_eq!(LICENSE_KEY, request.license_key);
        assert!(request.count.is_none());
    }

    #[test]
    fn deserialize_request_optional() {
        let input = format!("{{\"customerId\":\"{CUSTOMER_ID}\",\"vesselId\":\"{VESSEL_ID}\",\"licenseKey\":\"{LICENSE_KEY}\",\"count\":{COUNT}}}");
        let request: Request = from_str(&input).unwrap();

        assert_eq!(CUSTOMER_ID, request.customer_id);
        assert_eq!(Some(COUNT), request.count);
    }
}
