#!/usr/bin/env cargo
use icon_api::openapi::IconApiV1Doc;

fn main() {
    let openapi = IconApiV1Doc::openapi();
    let json = serde_json::to_string_pretty(&openapi)
        .expect("Failed to serialize OpenAPI spec to JSON");

    println!("{}", json);
}
