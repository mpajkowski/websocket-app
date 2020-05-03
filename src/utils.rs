use anyhow::Result;
use serde_json::{json, Value};
use std::future::Future;
use tokio::task::JoinHandle;

/// Spawns future in background and logs errors received from running tasks
///
/// # Arguments:
/// * `fut` - a future with output `Result<()>`, threadsafe and borrowed forever
pub fn spawn_and_log_err<F>(fut: F) -> JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send +  'static,
{
    tokio::spawn(async move {
        if let Err(e) = fut.await {
            log::error!("an error occured: {}", e);
        }
    })
}

/// Creates incremental diff of two json documents
///
/// # Arguments:
/// * `old_state` - old document (will be modified inplace)
/// * `new_state` - new document
pub fn create_json_snapshot(old_state: &mut Value, new_state: &Value) {
    // equal: just return
    if old_state == new_state {
        std::mem::replace(old_state, json!({}));
        return;
    }

    let old_state = old_state.as_object_mut().unwrap();
    let new_state = new_state.as_object().unwrap();

    // remove keys
    let to_remove = old_state
        .keys()
        .filter_map(|key| {
            if !new_state.contains_key(key.as_str()) {
                Some(key.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for key in to_remove {
        old_state.remove(&key);
    }

    for (channel, value) in new_state.iter() {
        // iterate through keys and check if values should be removed/updated
        let old_value = match old_state.get_mut(&*channel) {
            Some(v) => v,
            None => {
                // insert new channel
                old_state.insert(channel.clone(), value.clone());
                continue;
            }
        };

        let new_dict = match value.as_object() {
            Some(v) => v,
            None => {
                // replace old value with new value
                *old_value = value.clone();
                continue;
            }
        };

        let old_dict = match old_value.as_object_mut() {
            Some(v) => v,
            None => {
                // upgrade new value to dict
                *old_value = Value::Object(new_dict.clone());
                continue;
            }
        };

        for (key, new_val) in new_dict.iter() {
            match old_dict.get_mut(&*key) {
                // remove equal values in order to reduce bandwidth
                Some(old_v) if old_v == new_val => {
                    old_dict.remove(&*key);
                }
                // update/append value under given key if changed/occurred
                Some(_) | None => {
                    old_dict.insert(key.clone(), new_val.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_patch() {
        let mut json1 = json!({"a": "xyz"});
        let json2 = json1.clone();
        create_json_snapshot(&mut json1, &json2);
        assert_eq!(json1, json!({}));

        let mut json1 = json!({"channel": {"a": "xyz"}});
        let json2 = json!({"channel": {"a": "abc"}});
        create_json_snapshot(&mut json1, &json2);
        assert_eq!(json1, json2);

        let mut json1 = json!({"channel": {"a": "xyz", "b": "notchangedwillberemoved"}});
        let json2 = json!({"channel": {"a": "abc", "b": "notchangedwillberemoved"}});
        create_json_snapshot(&mut json1, &json2);
        assert_eq!(json1, json!({"channel": {"a": "abc"}}));

        let mut json1 = json!({"channel": "usedtobeastring"});
        let json2 = json!({"channel": {"a": "abc", "b": "nowiamadict"}});
        create_json_snapshot(&mut json1, &json2);
        assert_eq!(json1, json2);

        let mut json1 = json!({"channel": {"a": "iusedtobeadict"}});
        let json2 = json!({"channel": "nowiamastring"});
        create_json_snapshot(&mut json1, &json2);
        assert_eq!(json1, json2);

        let mut json1 = json!({"channel_a": {"a": "xyz"}});
        let json2 = json!({"channel_b": {"a": "xyz"}});
        create_json_snapshot(&mut json1, &json2);
        assert_eq!(json1, json2);
    }
}
