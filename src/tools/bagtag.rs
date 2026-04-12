use chrono::Local;
use serde_json::{json, Value};
use std::path::PathBuf;

fn bagtag_path() -> Result<PathBuf, String> {
    let local_app_data = std::env::var("LOCALAPPDATA")
        .map_err(|_| "LOCALAPPDATA not set".to_string())?;
    Ok(PathBuf::from(local_app_data).join("CPC").join("config").join("bagtag.json"))
}

fn read_bagtag() -> Result<Value, String> {
    let path = bagtag_path()?;
    if !path.exists() {
        return Err("BagTag not configured - run installer".to_string());
    }
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read bagtag.json: {}", e))?;
    serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse bagtag.json: {}", e))
}

pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "bag_tag",
            "description": "Read install_code from bagtag.json and return it with connection timestamp appended",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "bag_read",
            "description": "Read and return full bagtag.json contents",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "bag_clear",
            "description": "Reset bagtag.json to empty defaults",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
    ]
}

pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "bag_tag" => bag_tag(args),
        "bag_read" => bag_read(args),
        "bag_clear" => bag_clear(args),
        _ => json!({"error": format!("Unknown bagtag tool: {}", name)}),
    }
}

fn bag_tag(_args: &Value) -> Value {
    match read_bagtag() {
        Ok(data) => {
            let install_code = data.get("install_code")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let timestamp = Local::now().format("%m%d%y%H%M%S").to_string();
            json!({ "tag": format!("{}-{}", install_code, timestamp) })
        }
        Err(e) => json!({"error": e}),
    }
}

fn bag_read(_args: &Value) -> Value {
    match read_bagtag() {
        Ok(data) => data,
        Err(e) => json!({"error": e}),
    }
}

fn bag_clear(_args: &Value) -> Value {
    match bagtag_path() {
        Ok(path) => {
            let empty = json!({
                "install_code": "",
                "install_date": "",
                "level": 0,
                "machine_id": ""
            });
            match std::fs::write(&path, serde_json::to_string_pretty(&empty).unwrap()) {
                Ok(_) => json!({"success": true, "message": "BagTag cleared"}),
                Err(e) => json!({"error": format!("Failed to write bagtag.json: {}", e)}),
            }
        }
        Err(e) => json!({"error": e}),
    }
}
