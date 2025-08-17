use crate::CONFIG;

use super::error::MqttProtocolError;

pub fn sub_topic_valid(topic: &str) -> bool {
    if topic.is_empty() || topic.len() > CONFIG.get().unwrap().mqtt.settings.max_topic_length {
        return false;
    }

    let mut chars = topic.chars();
    let mut prev_char = '\0';
    while let Some(c) = chars.next() {
        match c {
            '+' => {
                if prev_char != '/' && prev_char != '\0' {
                    return false;
                }
                if let Some(next_char) = chars.clone().next() {
                    if next_char != '/' {
                        return false;
                    }
                }
            }
            '#' => {
                if prev_char != '/' && prev_char != '\0' {
                    return false;
                }
                if chars.next().is_some() {
                    return false;
                }
            }
            _ => {}
        }
        prev_char = c;
    }

    true
}

pub fn pub_topic_valid(topic: &str) -> bool {
    if topic.is_empty() || topic.len() > 1024 {
        return false;
    }

    if topic.contains('+') || topic.contains('#') {
        return false;
    }

    true
}

pub fn is_shared_subscription(topic: &str) -> bool {
    topic.starts_with("$share/")
}

pub fn parse_shared_subscription(topic: &str) -> Result<(&str, &str), MqttProtocolError> {
    if !topic.starts_with("$share/") {
        return Err(MqttProtocolError::InvalidTopicFilter);
    }

    let parts: Vec<&str> = topic.splitn(3, '/').collect();
    if parts.len() != 3 || parts[1].is_empty() || parts[2].is_empty() {
        return Err(MqttProtocolError::InvalidTopicFilter);
    }

    Ok((parts[1], parts[2]))
}

pub fn generate_random_client_id() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                abcdefghijklmnopqrstuvwxyz\
                                0123456789";
    let mut rng = rand::rng();

    let random_suffix: String = (0..14)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    format!("inner_{}", random_suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_subscription() {
        let topic = "$share/group/topic";
        let result = parse_shared_subscription(topic);
        assert!(result.is_ok());
        let (group, actual_topic) = result.unwrap();
        assert_eq!(group, "group");
        assert_eq!(actual_topic, "topic");

        let topic2 = "$share/mygroup/sensors/temperature";
        let result2 = parse_shared_subscription(topic2);
        assert!(result2.is_ok());
        let (group2, actual_topic2) = result2.unwrap();
        assert_eq!(group2, "mygroup");
        assert_eq!(actual_topic2, "sensors/temperature");

        let invalid_topic = "$share//topic";
        assert!(parse_shared_subscription(invalid_topic).is_err());

        let invalid_topic2 = "$share/group/";
        assert!(parse_shared_subscription(invalid_topic2).is_err());

        let invalid_topic3 = "normal/topic";
        assert!(parse_shared_subscription(invalid_topic3).is_err());
    }
}
