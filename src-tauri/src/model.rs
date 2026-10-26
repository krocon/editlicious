//! DTOs shared with the frontend (serde camelCase). Keep in sync with the `*.models.ts` files
//! under `src/app/core/` by hand; every change on both sides + a test.

use serde::Serialize;

/// Data for the About dialog (`src/app/core/about/about.models.ts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub os: String,
    pub arch: String,
}
