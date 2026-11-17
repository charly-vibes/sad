// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 sad contributors

//! Purpose: allow a licence banner above the dogfood header.
//! Responsibilities: prove the check scans past Exemption-class leading comments.
//! Rationale: real files carry a licence block before their header.

pub fn licensed() -> bool {
    true
}