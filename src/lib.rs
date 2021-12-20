// Copyright (c) 2021 Ivan Jelincic <parazyd@dyne.org>
//
// This file is part of streamflow-finance/timelock-crate
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License version 3
// as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! The code providing timelock primitives
//! used by [streamflow.finance](https://streamflow.finance).

/// Entrypoint
#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;
/// Errors
pub(crate) mod error;
/// Structs and data
pub mod state;
/// Functions related to SPL tokens
//pub mod token;
/// Utility functions
pub(crate) mod utils;

pub(crate) mod cancel_stream;
pub(crate) mod create_stream;
pub(crate) mod stream_safety;
pub(crate) mod topup_stream;
pub(crate) mod transfer_recipient;
pub(crate) mod withdraw_stream;

pub(crate) const STRM_TREASURY: &str = "Ht5G1RhkcKnpLVLMhqJc5aqZ4wYUEbxbtZwGCVbgU7DL";

pub(crate) const MAX_STRING_SIZE: usize = 200;
