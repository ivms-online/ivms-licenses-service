/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

#![feature(async_closure, future_join)]

use aws_config::load_from_env;
use aws_sdk_dynamodb::types::AttributeValue::{L, M, N, S};
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_lambda::error::SdkError;
use aws_sdk_lambda::operation::invoke::{InvokeError, InvokeOutput};
use aws_sdk_lambda::Client as LambdaClient;
use aws_smithy_types::Blob;
use cucumber::{given, then, when, World};
use futures::future::join_all;
use serde_json::{from_slice, json, to_vec, Value};
use std::collections::HashMap;
use std::env::{var, VarError};
use std::future::join;
use tokio::main as tokio_main;

macro_rules! serialize_blob {
    ($($data:tt)+) => {
        Blob::new(
            to_vec(&json!($($data)+)).unwrap()
        )
    };
}

#[derive(World, Debug)]
#[world(init = Self::new)]
struct TestWorld {
    // initialization scope
    licenses_table: String,
    creator_lambda: String,
    deleter_lambda: String,
    fetcher_lambda: String,
    lister_lambda: String,
    dynamodb: DynamoDbClient,
    lambda: LambdaClient,
    // test run scope
    cleanup_keys: Vec<(String, String, String)>,
    invoke_response: Option<Result<InvokeOutput, SdkError<InvokeError>>>,
    customer_id: Option<String>,
    vessel_id: Option<String>,
    license_key: Option<String>,
}

impl TestWorld {
    async fn new() -> Result<Self, VarError> {
        let config = &load_from_env().await;

        Ok(Self {
            licenses_table: var("LICENSES_TABLE")?,
            creator_lambda: var("CREATOR_LAMBDA")?,
            deleter_lambda: var("DELETER_LAMBDA")?,
            fetcher_lambda: var("FETCHER_LAMBDA")?,
            lister_lambda: var("LISTER_LAMBDA")?,
            dynamodb: DynamoDbClient::new(config),
            lambda: LambdaClient::new(config),
            cleanup_keys: vec![],
            invoke_response: None,
            customer_id: None,
            vessel_id: None,
            license_key: None,
        })
    }
}

async fn delete_license(
    world: &TestWorld,
    customer_id: &Option<String>,
    vessel_id: &Option<String>,
    license_key: &Option<String>,
) {
    if let (Some(customer_id), Some(vessel_id), Some(license_key)) = (customer_id, vessel_id, license_key) {
        world
            .dynamodb
            .delete_item()
            .table_name(world.licenses_table.as_str())
            .key("customerAndVesselId", S(format!("{customer_id}:{vessel_id}")))
            .key("licenseKey", S(license_key.clone()))
            .send()
            .await
            .unwrap();
    }
}

async fn list_licenses(
    world: &TestWorld,
    customer_id: String,
    vessel_id: String,
    page_token: Option<String>,
) -> Result<InvokeOutput, SdkError<InvokeError>> {
    world
        .lambda
        .invoke()
        .function_name(world.lister_lambda.to_string())
        .payload(serialize_blob!({
            "customerId": customer_id,
            "vesselId": vessel_id,
            "pageToken": page_token,
        }))
        .send()
        .await
}

fn extract_list(response: &Option<Result<InvokeOutput, SdkError<InvokeError>>>) -> Vec<Value> {
    let response: HashMap<String, Value> = from_slice(
        response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    response["licenses"].as_array().unwrap().to_owned()
}

#[tokio_main]
async fn main() {
    TestWorld::cucumber()
        .after(|_feature, _rule, _scenario, _finished, world| {
            Box::pin(async move {
                if let Some(&mut ref cleanup) = world {
                    let tasks = cleanup.cleanup_keys.iter().map(async move |key| {
                        delete_license(
                            &cleanup,
                            &Some(key.0.clone()),
                            &Some(key.1.clone()),
                            &Some(key.2.clone()),
                        )
                        .await
                    });

                    join!(
                        join_all(tasks),
                        delete_license(&cleanup, &cleanup.customer_id, &cleanup.vessel_id, &cleanup.license_key),
                    )
                    .await;
                }
            })
        })
        .run_and_exit("tests/features")
        .await;
}

// Given …

#[given(
    expr = "There is a license {string} for vessel {string} of customer {string} with count {int} and expiration date {string}"
)]
async fn there_is_a_license(
    world: &mut TestWorld,
    license_key: String,
    vessel_id: String,
    customer_id: String,
    count: usize,
    expires_at: String,
) {
    world
        .cleanup_keys
        .push((customer_id.clone(), vessel_id.clone(), license_key.clone()));

    world
        .dynamodb
        .put_item()
        .table_name(world.licenses_table.as_str())
        .item("customerAndVesselId", S(format!("{customer_id}:{vessel_id}")))
        .item("customerId", S(customer_id))
        .item("vesselId", S(vessel_id))
        .item("licenseKey", S(license_key))
        .item("count", N(count.to_string()))
        .item("expiresAt", S(expires_at))
        .send()
        .await
        .unwrap();
}

#[given(expr = "There is no license {string} for vessel {string} of customer {string}")]
async fn there_is_no_license(world: &mut TestWorld, license_key: String, vessel_id: String, customer_id: String) {
    delete_license(world, &Some(customer_id), &Some(vessel_id), &Some(license_key)).await;
}

// When …

#[when(expr = "I delete license {string} for vessel {string} of customer {string}")]
async fn i_delete_license(world: &mut TestWorld, license_key: String, vessel_id: String, customer_id: String) {
    world.invoke_response = Some(
        world
            .lambda
            .invoke()
            .function_name(world.deleter_lambda.to_string())
            .payload(serialize_blob!({
                "customerId": customer_id,
                "vesselId": vessel_id,
                "licenseKey": license_key,
            }))
            .send()
            .await,
    );
}

#[when(
    expr = "I create license {string} for vessel {string} of customer {string} with count {int} and expiration date {string}"
)]
async fn i_create_vessel(
    world: &mut TestWorld,
    license_key: String,
    vessel_id: String,
    customer_id: String,
    count: usize,
    expires_at: String,
) {
    world.invoke_response = Some(
        world
            .lambda
            .invoke()
            .function_name(world.creator_lambda.to_string())
            .payload(serialize_blob!({
                "customerId": customer_id,
                "vesselId": vessel_id,
                "licenseKey": license_key,
                "count": count,
                "expiresAt": expires_at,
            }))
            .send()
            .await,
    );

    world.customer_id = Some(customer_id);
    world.vessel_id = Some(vessel_id);
}

#[when(expr = "I fetch license {string} for vessel {string} of customer {string}")]
async fn i_fetch_license(world: &mut TestWorld, license_key: String, vessel_id: String, customer_id: String) {
    world.invoke_response = Some(
        world
            .lambda
            .invoke()
            .function_name(world.fetcher_lambda.to_string())
            .payload(serialize_blob!({
                "customerId": customer_id,
                "vesselId": vessel_id,
                "licenseKey": license_key,
            }))
            .send()
            .await,
    );
}

#[when(expr = "I list licenses of for vessel {string} customer {string}")]
async fn i_list_licenses(world: &mut TestWorld, vessel_id: String, customer_id: String) {
    world.invoke_response = Some(list_licenses(world, customer_id, vessel_id, None).await);
}

#[when(expr = "I list licenses of for vessel {string} customer {string} with page token {string}")]
async fn i_list_vessels_page(world: &mut TestWorld, vessel_id: String, customer_id: String, page_token: String) {
    world.invoke_response = Some(list_vessels(world, customer_id, vessel_id, Some(page_token)).await);
}

// Then …

#[then(expr = "License {string} for vessel {string} of customer {string} does not exist")]
async fn license_does_not_exist(world: &mut TestWorld, license_key: String, vessel_id: String, customer_id: String) {
    assert!(world
        .dynamodb
        .get_item()
        .table_name(world.licenses_table.as_str())
        .item("customerAndVesselId", S(format!("{customer_id}:{vessel_id}")))
        .item("licenseKey", S(license_key))
        .send()
        .await
        .unwrap()
        .item
        .is_none())
}

#[then(expr = "I get {string} API error response")]
async fn i_get_api_error(world: &mut TestWorld, message: String) {
    let response: HashMap<String, String> = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    assert_eq!(message, response["errorMessage"]);
}

#[then(expr = "I can read license key as {string}")]
async fn i_can_read_license_key(world: &mut TestWorld, key: String) {
    let response: HashMap<String, Value> = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    assert_eq!(key.as_str(), response["licenseKey"].as_str().unwrap());
}

#[then(expr = "I can read license count as {int}")]
async fn i_can_read_license_count(world: &mut TestWorld, count: usize) {
    let response: HashMap<String, Value> = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    assert_eq!(count, response["count"].as_str().unwrap());
}

#[then(expr = "I can read license expiration date as {string}")]
async fn i_can_read_license_expiration_date(world: &mut TestWorld, expires_at: String) {
    let response: HashMap<String, Value> = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    assert_eq!(expires_at.as_str(), response["expiresAt"].as_str().unwrap());
}

#[then("I can read license key")]
async fn i_can_read_license_key_after_create(world: &mut TestWorld) {
    let response: String = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    world.license_key = Some(response);
    assert!(world.license_key.is_some());
}

#[then(expr = "License with that key exists with count {int} and expiration date {string}")]
async fn license_with_that_key_exists(world: &mut TestWorld, count: usize, expires_at: String) {
    let customer_id = world.customer_id.unwrap();
    let vessel_id = world.vessel_id.unwrap();

    let license = world
        .dynamodb
        .get_item()
        .table_name(world.licenses_table.as_str())
        .key("customerAndVesselId", S(format!("{customer_id}:{vessel_id}")))
        .key("licenseKey", S(world.license_key.clone().unwrap()))
        .send()
        .await
        .unwrap()
        .item;
    assert!(license.is_some());
    assert_eq!(
        count.to_string(),
        *license.as_ref().and_then(|item| item["count"].as_n().ok()).unwrap()
    );
    assert_eq!(
        expires_at,
        *license.as_ref().and_then(|item| item["expiresAt"].as_s().ok()).unwrap()
    );
}

#[then(expr = "I can read list of {int} licenses")]
async fn i_can_read_list_of_licenses(world: &mut TestWorld, count: usize) {
    let licenses = extract_list(&world.invoke_response);

    assert_eq!(count, licenses.len());
}

#[then(expr = "License at position {int} has name {string}")]
async fn license_at_position_has_key(world: &mut TestWorld, position: usize, key: String) {
    let licenses = extract_list(&world.invoke_response);

    assert_eq!(
        key,
        licenses[position].as_object().unwrap()["licenseKey"].as_str().unwrap()
    );
}
