/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

#![feature(future_join)]

mod api_error;
mod lambda;
mod license_dao;
mod model;
mod runtime_error;

pub use crate::api_error::ApiError;
pub use crate::lambda::run_lambda;
pub use crate::license_dao::LicenseDao;
pub use crate::model::{DynamoResultsPage, License};
pub use crate::runtime_error::RuntimeError;
