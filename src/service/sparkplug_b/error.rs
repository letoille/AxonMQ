use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpbError {
    #[error("Invalid Topic")]
    InvalidTopic,
    #[error("Invalid Payload")]
    InvalidPayload,
    #[error("Invalid Metric")]
    InvalidMetric,
    #[error("Invalid Metric Property")]
    InvalidMetricProperty,
    #[error("Invalid Sequence")]
    InvalidSeq,
    #[error("Invalid bdSeq")]
    InvalidbdSeq,
    #[error("bdSeq Is None")]
    BdSeqIsNone,
    #[error("bdSeq Not Found")]
    BdSeqNotFound,
    #[error("Invalid DataType")]
    InvalidDataType,
    #[error("Invalid Value")]
    InvalidValue,
    #[error("Invalid Node Rebirth")]
    InvalidNodeRebirth,
    #[error("Invalid TimeStamp")]
    InvalidTimeStamp,
    #[error("Invalid QoS")]
    InvalidQoS,
    #[error("Invalid Retain")]
    InvalidRetain,
    #[error("Invalid Template")]
    InvalidTemplate,
    #[error("Invalid Template Instance")]
    InvalidTemplateInstance,
    #[error("Template Not Found")]
    TemplateNotFound,
    #[error("Invalid PropertySet")]
    InvalidPropertySet,
    #[error("Template Version Mismatch")]
    TemplateVersionMismatch,
    #[error("Exceeded")]
    Exceeded,
    #[error("Not Support Command")]
    NotSupportCommand,
    #[error("Node Death Not Match")]
    NDeathNotMatch,
    #[error("Node Not Found")]
    NodeNotFound,
    #[error("Group Not Found")]
    GroupNotFound,
    #[error("Device Not Found")]
    DeviceNotFound,

    // need birth
    #[error("Node Not Birth")]
    NodeNotBirth,
    #[error("Device Not Birth")]
    DeviceNotBirth,
    #[error("Metric Not Found")]
    MetricNotFound,
    #[error("Metric Not Match")]
    MetricNotMatch,
}
