/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

#![feature(future_join)]

use chrono::{DateTime, FixedOffset};
use lambda_runtime::{Error, LambdaEvent};
use licenses_core::{run_lambda, DynamoResultsPage, License, LicenseDao};
use serde::{Deserialize, Serialize};
use tokio::main as tokio_main;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    customer_id: Uuid,
    vessel_id: Uuid,
    page_token: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LicenseResponse {
    license_key: String,
    count: Option<u8>,
    expires_at: Option<DateTime<FixedOffset>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    licenses: Vec<LicenseResponse>,
    page_token: Option<String>,
}

impl From<License> for LicenseResponse {
    fn from(model: License) -> Self {
        Self {
            license_key: model.license_key,
            count: model.count,
            expires_at: model.expires_at,
        }
    }
}

impl From<DynamoResultsPage<License, String>> for Response {
    fn from(value: DynamoResultsPage<License, String>) -> Self {
        Self {
            licenses: value.items.into_iter().map(LicenseResponse::from).collect(),
            page_token: value.last_evaluated_key,
        }
    }
}

#[tokio_main]
async fn main() -> Result<(), Error> {
    let dao = &LicenseDao::load_from_env().await?;

    run_lambda!(move |event: LambdaEvent<Request>| async move {
        dao.list_licenses(
            event.payload.customer_id,
            event.payload.vessel_id,
            event.payload.page_token,
        )
        .await
        .map(Response::from)
    })
}

#[cfg(test)]
mod tests {
    use crate::{LicenseResponse, Request, Response};
    use licenses_core::{DynamoResultsPage, License};
    use serde_json::{from_str, to_string};
    use uuid::{uuid, Uuid};

    const CUSTOMER_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000000");
    const VESSEL_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
    const LICENSE_KEY: &str = "Test0";
    const COUNT: u8 = 42;
    const PAGE_TOKEN: &str = "abc";

    #[test]
    fn deserialize_request() {
        let input =
            format!("{{\"customerId\":\"{CUSTOMER_ID}\",\"vesselId\":\"{VESSEL_ID}\",\"pageToken\":\"{PAGE_TOKEN}\"}}");
        let request: Request = from_str(&input).unwrap();

        assert_eq!(CUSTOMER_ID, request.customer_id);
        assert_eq!(VESSEL_ID, request.vessel_id);
        assert_eq!(Some(PAGE_TOKEN.to_string()), request.page_token);
    }

    #[test]
    fn deserialize_request_no_page() {
        let input = format!("{{\"customerId\":\"{CUSTOMER_ID}\",\"vesselId\":\"{VESSEL_ID}\"}}");
        let request: Request = from_str(&input).unwrap();

        assert_eq!(CUSTOMER_ID, request.customer_id);
        assert_eq!(VESSEL_ID, request.vessel_id);
        assert!(request.page_token.is_none());
    }

    #[test]
    fn serialize_response() {
        let output = to_string(&Response {
            licenses: vec![LicenseResponse {
                license_key: LICENSE_KEY.to_string(),
                count: Some(COUNT),
                expires_at: None,
            }],
            page_token: Some(PAGE_TOKEN.to_string()),
        })
        .unwrap();

        assert!(output.contains(&format!("{COUNT}")));
        assert!(output.contains(&format!("\"{PAGE_TOKEN}\"")));
    }

    #[test]
    fn serialize_response_no_page() {
        let output = to_string(&Response {
            licenses: vec![LicenseResponse {
                license_key: LICENSE_KEY.to_string(),
                count: Some(COUNT),
                expires_at: None,
            }],
            page_token: None,
        })
        .unwrap();

        assert!(!output.contains(&format!("\"page_token\":")));
    }

    #[test]
    fn response_license_from_model() {
        let response = LicenseResponse::from(License {
            customer_id: CUSTOMER_ID,
            vessel_id: VESSEL_ID,
            license_key: LICENSE_KEY.to_string(),
            count: Some(COUNT),
            expires_at: None,
        });

        assert_eq!(Some(COUNT), response.count);
    }

    #[test]
    fn response_from_model() {
        let response = Response::from(DynamoResultsPage {
            items: vec![License {
                customer_id: CUSTOMER_ID,
                vessel_id: VESSEL_ID,
                license_key: LICENSE_KEY.to_string(),
                count: Some(COUNT),
                expires_at: None,
            }],
            last_evaluated_key: Some(PAGE_TOKEN.to_string()),
        });

        assert_eq!(1, response.licenses.len());
        assert_eq!(LICENSE_KEY, response.licenses[0].license_key);
        assert_eq!(Some(PAGE_TOKEN.to_string()), response.page_token);
    }
}
