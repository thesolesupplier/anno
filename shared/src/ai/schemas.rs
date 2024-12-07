use serde_json::{json, Value};

pub fn release_summary_response() -> Value {
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": "json_objects_array",
            "schema": {
                "type": "object",
                "properties": {
                    "items": {
                        "type": "array",
                        "description": "An array of JSON objects where each object has a title and an items array.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": {
                                    "type": "string",
                                    "description": "The title of the JSON object."
                                },
                                "items": {
                                    "type": "array",
                                    "description": "An array of strings.",
                                    "items": {
                                        "type": "string"
                                    }
                                }
                            },
                            "required": [
                                "title",
                                "items"
                            ],
                            "additionalProperties": false
                        }
                    }
                },
                "required": [
                    "items"
                ],
                "additionalProperties": false
            },
            "strict": true
        }
    })
}

pub fn test_cases_response() -> Value {
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": "cases_schema",
            "schema": {
              "type": "object",
              "properties": {
                "cases": {
                  "type": "array",
                  "description": "An array of cases represented as strings.",
                  "items": {
                    "type": "string"
                  }
                }
              },
              "required": [
                "cases"
              ],
              "additionalProperties": false
            },
            "strict": true
          }
    })
}
