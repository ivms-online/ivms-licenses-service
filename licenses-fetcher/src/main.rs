/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

#![feature(future_join)]

use chrono::{DateTime, FixedOffset};
use lambda_runtime::{Error, LambdaEvent};
use licenses_core::{run_lambda, ApiError, License, LicenseDao};
use serde::{Deserialize, Serialize};
use tokio::main as tokio_main;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    customer_id: Uuid,
    vessel_id: Uuid,
    license_key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    license_key: String,
    count: Option<u8>,
    expires_at: Option<DateTime<FixedOffset>>,
}

impl From<License> for Response {
    fn from(model: License) -> Self {
        Self {
            license_key: model.license_key,
            count: model.count,
            expires_at: model.expires_at,
        }
    }
}

#[tokio_main]
async fn main() -> Result<(), Error> {
    let dao = &LicenseDao::load_from_env().await?;

    run_lambda!(move |event: LambdaEvent<Request>| async move {
        match dao
            .get_license(
                event.payload.customer_id,
                event.payload.vessel_id,
                event.payload.license_key.clone(),
            )
            .await?
        {
            None => Err(ApiError::LicenseNotFound(event.payload.license_key)),
            Some(license) => Ok(Response::from(license)),
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::{Request, Response};
    use licenses_core::License;
    use serde_json::{from_str, to_string};
    use uuid::{uuid, Uuid};

    const CUSTOMER_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000000");
    const VESSEL_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
    const LICENSE_KEY: &str = "tides.2023";
    const COUNT: u8 = 6;

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

    #[test]
    fn serialize_response() {
        let output = to_string(&Response {
            license_key: LICENSE_KEY.to_string(),
            count: Some(COUNT),
            expires_at: None,
        })
        .unwrap();

        assert!(output.contains(&format!("\"{LICENSE_KEY}\"")));
        assert!(output.contains(&format!("{COUNT}")));
    }

    #[test]
    fn response_from_model() {
        let response = Response::from(License {
            customer_id: CUSTOMER_ID,
            vessel_id: VESSEL_ID,
            license_key: LICENSE_KEY.to_string(),
            count: Some(COUNT),
            expires_at: None,
        });

        assert_eq!(LICENSE_KEY, response.license_key);
        assert_eq!(Some(COUNT), response.count);
        assert!(response.expires_at.is_none());
    }
}
