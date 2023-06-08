/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc = "License entity."]
pub struct License {
    #[doc = "Owner ID."]
    pub customer_id: Uuid,
    #[doc = "Vessel ID."]
    pub vessel_id: Uuid,
    #[doc = "License entry key."]
    pub license_key: String,
    #[doc = "Number of license activation."]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u8>,
    #[doc = "Date when license ends."]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<FixedOffset>>,
}

pub struct DynamoResultsPage<T, K> {
    pub items: Vec<T>,
    pub last_evaluated_key: Option<K>,
}
