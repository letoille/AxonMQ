pub fn ncmd_topic(group_id: &str, node_id: &str) -> String {
    format!("spBv1.0/{}/NCMD/{}", group_id, node_id)
}

pub fn dcmd_topic(group_id: &str, node_id: &str, device_id: &str) -> String {
    format!("spBv1.0/{}/DCMD/{}/{}", group_id, node_id, device_id)
}
