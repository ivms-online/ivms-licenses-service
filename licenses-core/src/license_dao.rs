/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

use crate::model::{DynamoResultsPage, License};
use crate::runtime_error::RuntimeError;
use std::collections::HashMap;

use aws_config::load_from_env;
use aws_sdk_dynamodb::types::AttributeValue::S;
use aws_sdk_dynamodb::Client;
use serde_dynamo::{from_item, from_items, to_item};
use std::env::var;
use tracing::{Instrument, Span};
use uuid::Uuid;
use xray::aws_metadata;

pub struct LicenseDao {
    client: Box<Client>,
    table_name: String,
}

#[inline(always)]
fn key_of(customer_id: &Uuid, vessel_id: &Uuid) -> String {
    format!("{customer_id}:{vessel_id}")
}

/**
Required environment variables:
<dl>
    <dt><code>LICENSES_TABLE</code></dt>
    <dd>Name of DynamoDB licenses table.</dd>
</dl>
 */
impl LicenseDao {
    pub async fn load_from_env() -> Result<Self, RuntimeError> {
        let config = &load_from_env().await;

        var("LICENSES_TABLE")
            .map(|table_name| {
                let client = Client::new(config);
                Self::new(client, table_name)
            })
            .map_err(RuntimeError::ClientConfigLoadingError)
    }

    pub fn new(client: Client, table_name: String) -> Self {
        Self {
            client: Box::new(client),
            table_name,
        }
    }

    pub async fn create_license(&self, license: License) -> Result<(), RuntimeError> {
        let key = key_of(&license.customer_id, &license.vessel_id);

        self.client
            .put_item()
            .table_name(self.table_name.as_str())
            .set_item(Some(to_item(license)?))
            .item("customerAndVesselId", S(key))
            .send()
            .instrument(self.instrumentation())
            .await?;
        Ok(())
    }

    pub async fn list_licenses(
        &self,
        customer_id: Uuid,
        vessel_id: Uuid,
        page_token: Option<String>,
    ) -> Result<DynamoResultsPage<License, String>, RuntimeError> {
        let key = key_of(&customer_id, &vessel_id);

        let results = self
            .client
            .query()
            .table_name(self.table_name.as_str())
            .key_condition_expression("customerAndVesselId = :customerAndVesselId")
            .expression_attribute_values(":customerAndVesselId", S(key.clone()))
            .set_exclusive_start_key(page_token.map(|license_key| {
                HashMap::from([
                    ("customerAndVesselId".into(), S(key)),
                    ("licenseKey".into(), S(license_key)),
                ])
            }))
            .send()
            .instrument(self.instrumentation())
            .await?;

        Ok(DynamoResultsPage {
            last_evaluated_key: results
                .last_evaluated_key()
                .and_then(|key| key["licenseKey"].as_s().ok())
                .cloned(),
            items: if let Some(items) = results.items {
                from_items(items)?
            } else {
                vec![]
            },
        })
    }

    pub async fn get_license(
        &self,
        customer_id: Uuid,
        vessel_id: Uuid,
        license_key: String,
    ) -> Result<Option<License>, RuntimeError> {
        self.client
            .get_item()
            .table_name(self.table_name.as_str())
            .key("customerAndVesselId", S(key_of(&customer_id, &vessel_id)))
            .key("licenseKey", S(license_key))
            .send()
            .instrument(self.instrumentation())
            .await?
            .item
            .map(from_item::<_, License>)
            .map_or(Ok(None), |license| license.map(Some))
            .map_err(RuntimeError::from)
    }

    pub async fn delete_license(
        &self,
        customer_id: Uuid,
        vessel_id: Uuid,
        license_key: String,
    ) -> Result<(), RuntimeError> {
        self.client
            .delete_item()
            .table_name(self.table_name.as_str())
            .key("customerAndVesselId", S(key_of(&customer_id, &vessel_id)))
            .key("licenseKey", S(license_key))
            .send()
            .instrument(self.instrumentation())
            .await?;
        Ok(())
    }

    fn instrumentation(&self) -> Span {
        aws_metadata(
            self.client.conf().region().map(|value| value.to_string()).as_deref(),
            Some(self.table_name.as_str()),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::license_dao::key_of;
    use crate::{License, LicenseDao, RuntimeError};
    use async_trait::async_trait;
    use aws_config::load_from_env;
    use aws_sdk_dynamodb::config::Builder;
    use aws_sdk_dynamodb::operation::put_item::{PutItemError, PutItemOutput};
    use aws_sdk_dynamodb::types::{
        AttributeDefinition,
        AttributeValue::{L, M, N, S},
        KeySchemaElement, KeyType, ProvisionedThroughput, ScalarAttributeType,
    };
    use aws_sdk_dynamodb::Client;
    use aws_smithy_http::result::SdkError;
    use chrono::{DateTime, FixedOffset, TimeZone, Utc};
    use std::collections::HashMap;
    use std::future::join;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use test_context::{test_context, AsyncTestContext};
    use tokio::test as tokio_test;
    use uuid::{uuid, Uuid};

    struct DynamoDbTestContext {
        client: Box<Client>,
        dao: Box<LicenseDao>,
        table_name: String,
    }

    static NUMBER: AtomicUsize = AtomicUsize::new(0);

    // customers
    static ID_0: Uuid = uuid!("00000000-0000-0000-0000-000000000000");
    // vessels
    static ID_1: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
    static ID_2: Uuid = uuid!("00000000-0000-0000-0000-000000000002");
    static ID_3: Uuid = uuid!("00000000-0000-0000-0000-000000000003");
    // licenses
    static LICENSE_KEY_0: &str = "tides.2023";
    static LICENSE_KEY_1: &str = "weather.2022";
    static LICENSE_KEY_2: &str = "dummy";

    #[async_trait]
    impl AsyncTestContext for DynamoDbTestContext {
        async fn setup() -> DynamoDbTestContext {
            let table_name = format!("Licenses{}", NUMBER.fetch_add(1, Ordering::SeqCst));
            let config = load_from_env().await;
            let local_config = Builder::from(&config).endpoint_url("http://localhost:8000").build();
            let client = Client::from_conf(local_config);

            client
                .create_table()
                .table_name(table_name.as_str())
                .attribute_definitions(
                    AttributeDefinition::builder()
                        .attribute_name("customerAndVesselId")
                        .attribute_type(ScalarAttributeType::S)
                        .build(),
                )
                .attribute_definitions(
                    AttributeDefinition::builder()
                        .attribute_name("licenseKey")
                        .attribute_type(ScalarAttributeType::S)
                        .build(),
                )
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("customerAndVesselId")
                        .key_type(KeyType::Hash)
                        .build(),
                )
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("licenseKey")
                        .key_type(KeyType::Range)
                        .build(),
                )
                .provisioned_throughput(
                    ProvisionedThroughput::builder()
                        .read_capacity_units(1000)
                        .write_capacity_units(1000)
                        .build(),
                )
                .send()
                .await
                .unwrap();

            let context = DynamoDbTestContext {
                client: Box::new(client.clone()),
                dao: Box::new(LicenseDao::new(client, table_name.clone())),
                table_name: table_name.clone(),
            };

            let (res1, res2, res3) = join!(
                context.create_record(&ID_0, &ID_1, LICENSE_KEY_0, Some(2), None),
                context.create_record(&ID_0, &ID_1, LICENSE_KEY_1, None, Some("2011-01-30T14:58:00+01:00")),
                context.create_record(&ID_0, &ID_2, LICENSE_KEY_0, Some(2), None),
            )
            .await;

            res1.unwrap();
            res2.unwrap();
            res3.unwrap();

            context
        }

        async fn teardown(self) {
            self.client
                .delete_table()
                .table_name(self.table_name)
                .send()
                .await
                .unwrap();
        }
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn create_license(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let expires_at = Utc
            .with_ymd_and_hms(2015, 7, 2, 1, 20, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(7200).unwrap());

        let save = ctx
            .dao
            .create_license(License {
                customer_id: ID_0,
                vessel_id: ID_2,
                license_key: LICENSE_KEY_1.to_string(),
                count: None,
                expires_at: Some(expires_at),
            })
            .await;
        assert!(save.is_ok());

        let license = ctx
            .client
            .get_item()
            .table_name(ctx.table_name.as_str())
            .key("customerAndVesselId", S(key_of(&ID_0, &ID_2)))
            .key("licenseKey", S(LICENSE_KEY_1.to_string()))
            .send()
            .await?;
        assert!(license.item.is_some());
        assert_eq!(
            "2015-07-02T03:20:00+02:00",
            license.item.unwrap()["expiresAt"].as_s().unwrap()
        );

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn get_license(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let expires_at = Utc
            .with_ymd_and_hms(2011, 1, 30, 13, 58, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(3600).unwrap());

        let license = ctx
            .dao
            .get_license(ID_0, ID_1, LICENSE_KEY_1.to_string())
            .await?
            .unwrap();
        assert!(license.count.is_none());
        assert_eq!(Some(expires_at), license.expires_at);

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn get_license_unexisting(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let unexisting = ctx.dao.get_license(ID_0, ID_1, LICENSE_KEY_2.to_string()).await?;
        assert!(unexisting.is_none());

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn delete_license(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let result = ctx.dao.delete_license(ID_0, ID_1, LICENSE_KEY_0.to_string()).await;
        assert!(result.is_ok());

        let license = ctx
            .client
            .get_item()
            .table_name(ctx.table_name.as_str())
            .key("customerAndVesselId", S(key_of(&ID_0, &ID_1)))
            .key("licenseKey", S(LICENSE_KEY_0.to_string()))
            .send()
            .await?;
        assert!(license.item.is_none());

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn delete_license_unexisting(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let unexisting = ctx.dao.delete_license(ID_0, ID_1, LICENSE_KEY_2.to_string()).await;
        assert!(unexisting.is_ok());

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn list_licenses(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let unexisting = ctx.dao.list_licenses(ID_0, ID_1, None).await;
        assert!(unexisting.is_ok());

        let results = unexisting.unwrap();
        assert_eq!(2, results.items.len());
        assert_eq!(LICENSE_KEY_0, results.items[0].license_key);

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn list_licenses_page(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let unexisting = ctx.dao.list_licenses(ID_0, ID_1, Some(LICENSE_KEY_0.to_string())).await;
        assert!(unexisting.is_ok());

        let results = unexisting.unwrap();
        assert_eq!(1, results.items.len());
        assert_eq!(LICENSE_KEY_1, results.items[0].license_key);

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn list_licenses_unexisting(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let unexisting = ctx.dao.list_licenses(ID_1, ID_2, None).await;
        assert!(unexisting.is_ok());

        let results = unexisting.unwrap();
        assert!(results.items.is_empty());
        assert!(results.last_evaluated_key.is_none());

        Ok(())
    }

    impl DynamoDbTestContext {
        async fn create_record(
            &self,
            customer_id: &Uuid,
            vessel_id: &Uuid,
            license_key: &str,
            count: Option<u8>,
            expires_at: Option<&str>,
        ) -> Result<PutItemOutput, SdkError<PutItemError>> {
            let mut request = self
                .client
                .put_item()
                .table_name(self.table_name.as_str())
                .item("customerAndVesselId", S(key_of(customer_id, vessel_id)))
                .item("customerId", S(customer_id.to_string()))
                .item("vesselId", S(vessel_id.to_string()))
                .item("licenseKey", S(license_key.into()));

            if let Some(value) = count {
                request = request.item("count", N(value.to_string()));
            }
            if let Some(value) = expires_at {
                request = request.item("expiresAt", S(value.to_string()));
            }

            request.send().await
        }
    }
}
