pub fn topic_match(filter: &str, topic: &str) -> bool {
    let filter_parts: Vec<&str> = filter.split('/').collect();
    let topic_parts: Vec<&str> = topic.split('/').collect();

    let mut f_iter = filter_parts.iter();
    let mut t_iter = topic_parts.iter();

    while let Some(f_part) = f_iter.next() {
        match *f_part {
            "#" => return true,
            "+" => {
                if t_iter.next().is_none() {
                    return false;
                }
            }
            _ => {
                if let Some(t_part) = t_iter.next() {
                    if f_part != t_part {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
    }

    t_iter.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_match() {
        assert!(topic_match("sport/tennis/player1", "sport/tennis/player1"));
        assert!(topic_match("sport/+/player1", "sport/tennis/player1"));
        assert!(topic_match("sport/+/+", "sport/tennis/player1"));
        assert!(topic_match("sport/#", "sport/tennis/player1"));
        assert!(topic_match("#", "sport/tennis/player1"));
        assert!(!topic_match("sport/tennis/player1", "sport/tennis/player2"));
        assert!(!topic_match("sport/+/player1", "sport/tennis/player2"));
        assert!(!topic_match("sport/+/+", "sport/tennis"));
        assert!(!topic_match("sport/#", "news/tennis/player1"));
        assert!(!topic_match("sport", "sport/tennis/player1"));
    }
}
