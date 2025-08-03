use crate::CONFIG;

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
