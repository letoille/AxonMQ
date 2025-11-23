use warp::Filter;

use crate::service::sparkplug_b::in_helper::{InHelper as SpbInHelper, KV};

use super::error::ApiError;

use super::{decode_param, with_spb_in_helper};

pub async fn get_groups(spb_in_helper: SpbInHelper) -> Result<impl warp::Reply, warp::Rejection> {
    let result = spb_in_helper
        .get_groups(None)
        .await
        .map_err(|e| ApiError::from(e))?;
    Ok(warp::reply::json(&result))
}

pub async fn get_group(
    group_id: String,
    spb_in_helper: SpbInHelper,
) -> Result<impl warp::Reply, warp::Rejection> {
    let result = spb_in_helper
        .get_groups(Some(group_id))
        .await
        .map_err(|e| ApiError::from(e))?;
    Ok(warp::reply::json(&result[0]))
}

pub async fn get_nodes(
    group_id: String,
    spb_in_helper: SpbInHelper,
) -> Result<impl warp::Reply, warp::Rejection> {
    let result = spb_in_helper
        .get_nodes(group_id, None)
        .await
        .map_err(|e| ApiError::from(e))?;
    Ok(warp::reply::json(&result))
}

pub async fn get_node(
    group_id: String,
    node_id: String,
    spb_in_helper: SpbInHelper,
) -> Result<impl warp::Reply, warp::Rejection> {
    let result = spb_in_helper
        .get_nodes(group_id, Some(node_id))
        .await
        .map_err(|e| ApiError::from(e))?;
    Ok(warp::reply::json(&result[0]))
}

pub async fn get_devices(
    group_id: String,
    node_id: String,
    spb_in_helper: SpbInHelper,
) -> Result<impl warp::Reply, warp::Rejection> {
    let result = spb_in_helper
        .get_devices(group_id, node_id, None)
        .await
        .map_err(|e| ApiError::from(e))?;
    Ok(warp::reply::json(&result[0]))
}

pub async fn get_device(
    group_id: String,
    node_id: String,
    device: String,
    spb_in_helper: SpbInHelper,
) -> Result<impl warp::Reply, warp::Rejection> {
    let result = spb_in_helper
        .get_devices(group_id, node_id, Some(device))
        .await
        .map_err(|e| ApiError::from(e))?;
    Ok(warp::reply::json(&result[0]))
}

pub async fn set_node(
    group_id: String,
    node_id: String,
    kvs: Vec<KV>,
    spb_in_helper: SpbInHelper,
) -> Result<impl warp::Reply, warp::Rejection> {
    let result = spb_in_helper
        .set_node(group_id, node_id, kvs)
        .await
        .map_err(|e| ApiError::from(e))?;
    Ok(warp::reply::json(&result))
}

pub async fn set_device(
    group_id: String,
    node_id: String,
    device_id: String,
    kvs: Vec<KV>,
    spb_in_helper: SpbInHelper,
) -> Result<impl warp::Reply, warp::Rejection> {
    let result = spb_in_helper
        .set_device(group_id, node_id, device_id, kvs)
        .await
        .map_err(|e| ApiError::from(e))?;
    Ok(warp::reply::json(&result))
}

pub(crate) fn spb_routers(
    spb_in_helper: SpbInHelper,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let api_get_groups = warp::get()
        .and(warp::path!(
            "api" / "v1" / "services" / "sparkplug_b" / "groups"
        ))
        .and(with_spb_in_helper(spb_in_helper.clone()))
        .and_then(get_groups);

    let api_get_group = warp::get()
        .and(warp::path!(
            "api" / "v1" / "services" / "sparkplug_b" / "groups" / String
        ))
        .map(|group_id: String| decode_param(&group_id))
        .and(with_spb_in_helper(spb_in_helper.clone()))
        .and_then(get_group);

    let api_get_nodes = warp::get()
        .and(warp::path!(
            "api" / "v1" / "services" / "sparkplug_b" / "groups" / String / "nodes"
        ))
        .and(with_spb_in_helper(spb_in_helper.clone()))
        .and_then(get_nodes);

    let api_get_node = warp::get()
        .and(warp::path!(
            "api" / "v1" / "services" / "sparkplug_b" / "groups" / String / "nodes" / String
        ))
        .map(|group_id: String, node_id: String| (decode_param(&group_id), decode_param(&node_id)))
        .untuple_one()
        .and(with_spb_in_helper(spb_in_helper.clone()))
        .and_then(get_node);

    let api_get_devices = warp::get()
        .and(warp::path!(
            "api"
                / "v1"
                / "services"
                / "sparkplug_b"
                / "groups"
                / String
                / "nodes"
                / String
                / "devices"
        ))
        .map(|group_id: String, node_id: String| (decode_param(&group_id), decode_param(&node_id)))
        .untuple_one()
        .and(with_spb_in_helper(spb_in_helper.clone()))
        .and_then(get_devices);

    let api_get_device = warp::get()
        .and(warp::path!(
            "api"
                / "v1"
                / "services"
                / "sparkplug_b"
                / "groups"
                / String
                / "nodes"
                / String
                / "devices"
                / String
        ))
        .map(|group_id: String, node_id: String, device: String| {
            (
                decode_param(&group_id),
                decode_param(&node_id),
                decode_param(&device),
            )
        })
        .untuple_one()
        .and(with_spb_in_helper(spb_in_helper.clone()))
        .and_then(get_device);

    let api_set_node = warp::put()
        .and(warp::path!(
            "api" / "v1" / "services" / "sparkplug_b" / "groups" / String / "nodes" / String
        ))
        .map(|group_id: String, node_id: String| (decode_param(&group_id), decode_param(&node_id)))
        .untuple_one()
        .and(warp::body::json())
        .and(with_spb_in_helper(spb_in_helper.clone()))
        .and_then(set_node);

    let api_set_device = warp::put()
        .and(warp::path!(
            "api"
                / "v1"
                / "services"
                / "sparkplug_b"
                / "groups"
                / String
                / "nodes"
                / String
                / "devices"
                / String
        ))
        .map(|group_id: String, node_id: String, device: String| {
            (
                decode_param(&group_id),
                decode_param(&node_id),
                decode_param(&device),
            )
        })
        .untuple_one()
        .and(warp::body::json())
        .and(with_spb_in_helper(spb_in_helper.clone()))
        .and_then(set_device);

    api_get_groups
        .or(api_get_group)
        .or(api_get_nodes)
        .or(api_get_node)
        .or(api_get_devices)
        .or(api_get_device)
        .or(api_set_node)
        .or(api_set_device)
}
