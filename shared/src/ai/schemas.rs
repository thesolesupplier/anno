use serde_json::{json, Value};

pub fn release_summary_response() -> Value {
    let category_title = json!({
        "type": "string",
        "description": "The title of the JSON object."
    });

    let category_items = json!({
        "type": "array",
        "description": "An array of strings.",
        "items": {
            "type": "string"
        }
    });

    let category = json!({
        "type": "object",
        "properties": {
            "title": category_title,
            "items": category_items
        },
        "required": [
            "title",
            "items"
        ],
        "additionalProperties": false
    });

    let categories = json!({
        "type": "array",
        "description": "An array of JSON objects where each object has a title and an items array.",
        "items": category
    });

    let schema = json!({
        "name": "json_objects_array",
        "schema": {
            "type": "object",
            "properties": {
                "items": categories
            },
            "required": [
                "items"
            ],
            "additionalProperties": false
        },
        "strict": true
    });

    json!({
        "type": "json_schema",
        "json_schema": schema
    })
}

pub fn test_cases_response() -> Value {
    let test_case = json!({
      "type": "array",
      "description": "An array of cases represented as strings.",
      "items": {
        "type": "string"
      }
    });

    let schema = json!({
      "name": "cases_schema",
      "schema": {
        "type": "object",
        "properties": {
          "cases": test_case
        },
        "required": [
          "cases"
        ],
        "additionalProperties": false
      },
      "strict": true
    });

    json!({
        "type": "json_schema",
        "json_schema": schema
    })
}
