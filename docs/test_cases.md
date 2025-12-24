# MQTT Broker Core Functionality Test Cases

This document contains a series of test cases for verifying the core functionality of an MQTT v5.0 Broker. It has been expanded based on the official specification to cover more edge cases and critical scenarios. Each test case is associated with the corresponding section of the MQTT v5.0 specification.

---

## Part 1: Basic QoS Delivery (Online Scenarios)

**Objective**: To verify that the Broker correctly handles different QoS combinations, follows the `Effective QoS = MIN(Pub QoS, Sub QoS)` rule, and covers exceptions like network interruptions.

#### ✔️ **Case ID: QOS-1.1**
*   **Description**: Test QoS degradation when the published QoS is higher than the subscribed QoS.
*   **MQTT v5.0 Spec Reference**: Section `3.8.4.2 QoS level granted` - The QoS granted by the server is the minimum of the published QoS and the subscribed QoS.
*   **Setup**: Client A subscribes to topic `T1` (QoS 1). Client B publishes a message to `T1` (QoS 2).
*   **Expected Result**: Client A receives the message at QoS 1 and completes the QoS 1 acknowledgment flow.

#### ✔️ **Case ID: QOS-1.2**
*   **Description**: Test QoS degradation when the subscribed QoS is higher than the published QoS.
*   **MQTT v5.0 Spec Reference**: Section `3.8.4.2 QoS level granted` - (Same as above).
*   **Setup**: Client A subscribes to topic `T1` (QoS 2). Client B publishes a message to `T1` (QoS 1).
*   **Expected Result**: Client A receives the message at QoS 1 and completes the QoS 1 acknowledgment flow.

#### ✔️ **Case ID: QOS-1.3**
*   **Description**: Test QoS 0 delivery, even when the subscriber requests a higher QoS.
*   **MQTT v5.0 Spec Reference**: Section `3.8.4.2 QoS level granted` - (Same as above).
*   **Setup**: Client A subscribes to topic `T1` (QoS 2). Client B publishes a message to `T1` (QoS 0).
*   **Expected Result**: Client A receives the message at QoS 0 with no acknowledgment interaction.

#### ✔️ **Case ID: QOS-1.4**
*   **Description**: Test end-to-end QoS 2 perfect delivery.
*   **MQTT v5.0 Spec Reference**: Section `4.3.3 QoS 2: Exactly once delivery` - Describes the complete QoS 2 message exchange flow.
*   **Setup**: Client A subscribes to topic `T1` (QoS 2). Client B publishes a message to `T1` (QoS 2).
*   **Expected Result**: Client A receives the message at QoS 2 and completes the full QoS 2 four-part handshake (PUBLISH, PUBREC, PUBREL, PUBCOMP).

#### ✔️ **Case ID: QOS-1.5**
*   **Description**: Test the recovery capability of the QoS 2 delivery flow after a network interruption.
*   **MQTT v5.0 Spec Reference**: Section `4.3.3 QoS 2: Exactly once delivery` - The specification requires the sender and receiver to store intermediate states to ensure delivery.
*   **Setup**: Client A connects with `clean_start=false` and `Session Expiry Interval > 0`, and subscribes to `T1` (QoS 2). Client B publishes a QoS 2 message to `T1`.
*   **Action**: Broker sends `PUBLISH` -> Client A receives it and sends `PUBREC` -> Client A disconnects abnormally -> Client A reconnects before the session expires.
*   **Expected Result**: The Broker should resend `PUBREL` after reconnection, and the delivery is ultimately completed exactly once.

---

## Part 2: Session and Offline Messages

**Objective**: To verify how the Broker correctly handles sessions based on `clean_start` and `Session Expiry Interval`.

#### ✔️ **Case ID: SESS-2.1**
*   **Description**: Verify that a non-persistent session (`clean_start=true`) does not retain any state during the client's offline period.
*   **MQTT v5.0 Spec Reference**: Section `3.1.2.4 Clean Start` - If `Clean Start` is 1, the client and server must discard any existing session.
*   **Setup**: Client A connects with `clean_start=true` and subscribes to `T1` (QoS 1).
*   **Action**: Client A goes offline -> Client B publishes a message to `T1` -> Client A reconnects with `clean_start=true`.
*   **Expected Result**: Client A must re-subscribe and will not receive the offline message.

#### ✔️ **Case ID: SESS-2.2**
*   **Description**: Verify that a persistent session (`clean_start=false` and `Session Expiry Interval > 0`) can store QoS 1 messages for an offline client.
*   **MQTT v5.0 Spec Reference**: Sections `3.1.2.4 Clean Start`, `3.1.2.1.2 Session Expiry Interval`, `4.1 Session State` - If `Clean Start` is 0 and the session has not expired, the server must resume the session, whose state includes subscriptions and pending QoS 1/2 messages.
*   **Setup**: Client A connects with `clean_start=false` and `Session Expiry Interval > 0` (e.g., 300 seconds) and subscribes to `T1` (QoS 1).
*   **Action**: Client A goes offline -> Client B publishes a message to `T1` -> Client A reconnects with `clean_start=false` before the session expires.
*   **Expected Result**: Client A's subscription is still valid, and it receives the offline message.

#### ✔️ **Case ID: SESS-2.3**
*   **Description**: Verify that when `Session Expiry Interval` is 0, the session ends immediately after the connection is closed.
*   **MQTT v5.0 Spec Reference**: Section `3.1.2.1.2 Session Expiry Interval` - If the interval is 0, the session ends when the network connection is closed.
*   **Setup**: Client A connects with `clean_start=false` and `Session Expiry Interval = 0` and subscribes to `T1`.
*   **Action**: Client A goes offline -> Client B publishes a message -> Client A reconnects with `clean_start=false`.
*   **Expected Result**: Client A's session has ended, and it must re-subscribe.

#### ✔️ **Case ID: SESS-2.4**
*   **Description**: Verify that the session is correctly cleared after the `Session Expiry Interval` expires.
*   **MQTT v5.0 Spec Reference**: Section `3.1.2.1.2 Session Expiry Interval` - The session will last for the corresponding amount of time after the network connection is closed.
*   **Setup**: Client A connects with `clean_start=false` and `Session Expiry Interval = 5` seconds and subscribes to `T1`.
*   **Action**: Client A goes offline -> Wait for 10 seconds -> Client A reconnects with `clean_start=false`.
*   **Expected Result**: Client A's old session has expired, and this is a new session.

#### ✔️ **Case ID: SESS-2.5**
*   **Description**: Verify that a persistent session (`clean_start=false` and `Session Expiry Interval > 0`) does **not** store QoS 0 messages for an offline client.
*   **MQTT v5.0 Spec Reference**: Section `4.1 Session State` - The session state defined in the specification only includes QoS 1 and QoS 2 messages, not QoS 0.
*   **Setup**:
    *   Client A connects to the Broker with `clean_start=false`, `Session Expiry Interval > 0`, and a fixed ClientID, and subscribes to topic `T1`.
*   **Action**:
    *   1. Client A disconnects normally.
    *   2. Client B publishes a **QoS 0** message to topic `T1`.
    *   3. Client A reconnects before the session expires, using `clean_start=false` and the same ClientID.
*   **Expected Result**:
    *   Client A's subscription is still valid.
    *   Client A does **not** receive the QoS 0 message that was published while it was offline.

#### ✔️ **Case ID: SESS-2.6**
*   **Description**: Verify that the `Session Expiry Interval` in a `DISCONNECT` packet overrides the interval set during connection.
*   **MQTT v5.0 Spec Reference**: Section `3.14.2.2.2 Session Expiry Interval` - The value in `DISCONNECT` replaces the value established in `CONNECT`.
*   **Setup**: Client A connects with `clean_start=false` and `Session Expiry Interval = 300` seconds.
*   **Action**: Client A sends a `DISCONNECT` packet with `Session Expiry Interval = 0`.
*   **Expected Result**: The Broker must immediately discard the session state, not waiting for 300 seconds.

---

## Part 3: Message Lifecycle (Message Expiry)

**Objective**: To verify that the Broker correctly handles `Message Expiry Interval`.

#### ✔️ **Case ID: EXP-3.1**
*   **Description**: Verify that the Broker discards expired offline messages.
*   **MQTT v5.0 Spec Reference**: Section `3.3.2.3.3 Message Expiry Interval` - If the message has expired by the time it is to be delivered, the server must discard it.
*   **Setup**: Client A connects with `clean_start=false` and `Session Expiry Interval > 0`, subscribes to `T1` (QoS 1), and goes offline.
*   **Action**: Client B publishes a message to `T1` (Expiry = 5s) -> Wait for 10s -> Client A reconnects before the session expires.
*   **Expected Result**: Client A will **not** receive the message.

#### ✔️ **Case ID: EXP-3.2**
*   **Description**: Verify that an expired retained message is not sent to new subscribers.
*   **MQTT v5.0 Spec Reference**: Section `3.3.2.3.4 Message Expiry and Retained Messages` - The server must check if a retained message has expired before delivering it.
*   **Setup**: Client A publishes a retained message to `T_RETAIN_EXP` (Expiry = 5s).
*   **Action**: Wait for 10s -> Client B subscribes to `T_RETAIN_EXP`.
*   **Expected Result**: Client B will **not** receive any retained message.

#### ✔️ **Case ID: EXP-3.3**
*   **Description**: Verify that a message with `Message Expiry Interval` of 0 is not retained for a long time.
*   **MQTT v5.0 Spec Reference**: Section `3.3.1.3 RETAIN` - The spec notes that a message with RETAIN flag and zero-byte payload is used to clear retained messages, but for a message with Expiry=0, its nature is "send and delete".
*   **Setup**: Client A publishes a retained message to `T_RETAIN_EXP_ZERO` (Expiry = 0).
*   **Action**: Client B subscribes to `T_RETAIN_EXP_ZERO`.
*   **Expected Result**: Client B will **not** receive any retained message.

---

## Part 4: Last Will and Testament (LWT)

**Objective**: To verify that the Broker correctly handles various LWT scenarios.

#### ✔️ **Case ID: LWT-4.1**
*   **Description**: Verify that an abnormal client disconnection triggers the publication of the LWT message.
*   **MQTT v5.0 Spec Reference**: Section `3.1.2.5 Will Flag` - If the Will Flag is 1, the server must publish the Will Message when the network connection is not closed normally.
*   **Setup**: Client C subscribes to `T_LWT`. Client A sets up an LWT upon connection.
*   **Action**: Forcibly interrupt Client A's TCP connection.
*   **Expected Result**: Client C receives the LWT message.

#### ✔️ **Case ID: LWT-4.2**
*   **Description**: Verify that a normal client disconnection (`DISCONNECT`) does not trigger the LWT.
*   **MQTT v5.0 Spec Reference**: Section `3.14.2.1 Disconnecting Normally` - If the client sends a `DISCONNECT` packet, the server must not publish its Will Message.
*   **Setup**: (Same as above).
*   **Action**: Client A sends a `DISCONNECT` packet.
*   **Expected Result**: Client C will **not** receive the LWT message.

#### ✔️ **Case ID: LWT-4.3**
*   **Description**: Verify that the LWT message can be correctly published with QoS 2.
*   **MQTT v5.0 Spec Reference**: Section `3.1.3.2 Will Properties` - The `Will QoS` field specifies the QoS level for the Will Message.
*   **Setup**: Client C subscribes to `T_LWT` (QoS 2). Client A sets up an LWT with QoS 2 upon connection.
*   **Action**: Forcibly interrupt Client A's TCP connection.
*   **Expected Result**: Client C receives the LWT message at QoS 2 and completes the QoS 2 acknowledgment flow.

#### ✔️ **Case ID: LWT-4.4**
*   **Description**: Verify that properties of a Will Message (e.g., Message Expiry) are respected upon publication, especially when retained.
*   **MQTT v5.0 Spec Reference**: Section `3.1.3.2 Will Properties`.
*   **Setup**: Client A connects with an LWT for topic `T_LWT_PROP_RETAIN`. The Will has `RETAIN=true` and `Message Expiry Interval = 5` seconds.
*   **Action**: 1. Client A disconnects abnormally. 2. Wait for 10 seconds. 3. Client B subscribes to `T_LWT_PROP_RETAIN`.
*   **Expected Result**: The Broker publishes the Will Message and retains it. However, after 5 seconds, the retained Will Message expires. Client B should not receive any message upon subscription.

---

## Part 5: Retained Messages

**Objective**: To verify the Broker's correct handling of the `RETAIN` flag.

#### ✔️ **Case ID: RETAIN-5.1**
*   **Description**: Verify that new subscribers immediately receive existing retained messages.
*   **MQTT v5.0 Spec Reference**: Section `3.3.1.3 RETAIN` - When a new subscription is established, the server must deliver matching retained messages.
*   **Setup**: Client A publishes a retained message to `T_RETAIN` -> Afterwards, Client B subscribes to `T_RETAIN`.
*   **Expected Result**: Client B immediately receives the retained message.

#### ✔️ **Case ID: RETAIN-5.2**
*   **Description**: Verify that publishing an empty retained message can clear the retained message for that topic.
*   **MQTT v5.0 Spec Reference**: Section `3.3.1.3 RETAIN` - A message with the RETAIN flag set to 1 and a zero-byte payload is used to clear retained messages.
*   **Setup**: A retained message for `T_RETAIN` already exists in the Broker.
*   **Action**: Client A publishes an empty retained message to `T_RETAIN` -> Afterwards, Client B subscribes to `T_RETAIN`.
*   **Expected Result**: Client B will **not** receive any retained message.

#### ✔️ **Case ID: RETAIN-5.3**
*   **Description**: Verify that a cleared retained message is not delivered to existing subscribers.
*   **MQTT v5.0 Spec Reference**: Section `3.3.1.3 RETAIN` - The clearing operation is to delete the stored message, not to publish an empty message.
*   **Setup**: Client C is already subscribed to `T_RETAIN`. A retained message for `T_RETAIN` already exists in the Broker.
*   **Action**: Client A publishes an empty retained message to `T_RETAIN`.
*   **Expected Result**: Client C will **not** receive this empty message.

#### ✔️ **Case ID: RETAIN-5.4**
*   **Description**: Verify that the Broker does not resend retained messages upon session resumption.
*   **MQTT v5.0 Spec Reference**: Section `3.3.1.3 RETAIN` - Retained messages are sent only "when a new subscription is established". Session resumption (`Clean Start=0`) is not considered a new subscription.
*   **Setup**:
    *   1. Client A publishes a retained message to topic `T_RETAIN_RESUME`.
    *   2. Client B connects with `clean_start=false` and `Session Expiry Interval > 0`, and subscribes to `T_RETAIN_RESUME`.
    *   3. Client B receives the retained message, then disconnects.
*   **Action**:
    *   Client B reconnects to its existing session using `clean_start=false` before the session expires.
*   **Expected Result**:
    *   Client B will **not** receive the retained message again upon successful reconnection.

#### ✔️ **Case ID: RETAIN-5.5**
*   **Description**: Verify that when a new client subscribes to a topic with a retained message, the retained message is sent **only** to the new subscriber and not to existing subscribers of that topic.
*   **MQTT v5.0 Spec Reference**: Section `3.3.1.3 RETAIN` - The spec implies this by stating the message is sent "when a new subscription is established", targeting that specific subscription event.
*   **Setup**:
    *   1. Client A publishes a retained message to topic `T_RETAIN_BROADCAST`.
    *   2. Client B subscribes to `T_RETAIN_BROADCAST` and receives the retained message. Client B remains connected and subscribed.
*   **Action**:
    *   A new client, Client C, subscribes to `T_RETAIN_BROADCAST`.
*   **Expected Result**:
    *   Client C receives the retained message.
    *   Client B (the existing subscriber) does **not** receive the message again.

---

## Part 6: Shared Subscriptions

**Objective**: To verify the Broker's load balancing for shared subscriptions and its interaction with other subscription types.

#### ✔️ **Case ID: SHARE-6.1**
*   **Description**: Verify that messages in a shared subscription group are received by only one member.
*   **MQTT v5.0 Spec Reference**: Section `4.8 Shared Subscriptions` - The server should select only one session to forward the message.
*   **Setup**: Client A and Client B both subscribe to the shared topic `$share/group1/T_SHARE`.
*   **Action**: Client C publishes a message to `T_SHARE`.
*   **Expected Result**: Only one of Client A or Client B receives the message.

#### ✔️ **Case ID: SHARE-6.2**
*   **Description**: Verify that shared subscriptions and non-shared subscriptions can coexist.
*   **MQTT v5.0 Spec Reference**: Section `4.8 Shared Subscriptions` - Shared subscriptions should not affect the distribution of non-shared subscriptions.
*   **Setup**: Client A subscribes to `$share/group1/T_SHARE`. Client B subscribes to `T_SHARE`.
*   **Action**: Client C publishes a message to `T_SHARE`.
*   **Expected Result**: Both Client A (or a member of its group) and Client B receive the message.

---

## Part 7: Will Delay Interval

**Objective**: To verify the Broker's support for the LWT delayed sending feature.

#### ✔️ **Case ID: WILL-DELAY-7.1**
*   **Description**: Verify that reconnecting within the delay interval can successfully cancel the sending of the Will Message.
*   **MQTT v5.0 Spec Reference**: Section `3.1.3.2.3 Will Delay Interval` - If the client reconnects within the delay interval, the server must not send the Will Message.
*   **Setup**: Client C subscribes to `T_LWT`. Client A connects with LWT set (Delay = 5s).
*   **Action**: Client A disconnects abnormally -> Reconnects after 3 seconds.
*   **Expected Result**: Client C will **not** receive any Will Message.

#### ✔️ **Case ID: WILL-DELAY-7.2**
*   **Description**: Verify that the Will Message is correctly sent after the delay interval expires.
*   **MQTT v5.0 Spec Reference**: Section `3.1.3.2.3 Will Delay Interval` - (Same as above).
*   **Setup**: (Same as above).
*   **Action**: Client A disconnects abnormally -> Wait for 7 seconds.
*   **Expected Result**: Client C **will** receive the Will Message.

---

## Part 8: Topic Alias

**Objective**: To verify that the Broker correctly handles and maps topic aliases.

#### ✔️ **Case ID: ALIAS-8.1**
*   **Description**: Verify that the Broker can correctly route messages using a topic alias set by the client.
*   **MQTT v5.0 Spec Reference**: Section `3.3.2.3.2 Topic Alias` - The client can use a topic alias instead of the topic name.
*   **Setup**: Client C subscribes to a long topic name.
*   **Action**: Client A publishes with the long topic name and alias 1 -> Client A publishes with an empty topic name and alias 1.
*   **Expected Result**: Client C receives two messages.

#### ✔️ **Case ID: ALIAS-8.2**
*   **Description**: Verify when the client attempts to establish more aliases than the Broker's `Topic Alias Maximum` limit, the Broker correctly handles it.
*   **MQTT v5.0 Spec Reference**: Sections `3.1.2.11.2 Topic Alias` & `3.2.2.3.4 Topic Alias Maximum`.
*   **Setup**: Broker declares `Topic Alias Maximum = 10` in `CONNACK`.
*   **Action**:
    1.  Client A establishes 10 unique topic-to-alias mappings by sending 10 `PUBLISH` packets, each with a unique topic (`topic/1` to `topic/10`) and a unique alias (`1` to `10`).
    2.  Client A then sends an 11th `PUBLISH` packet with a new topic (`topic/11`) and a new alias (`11`).
*   **Expected Result**: Broker should disconnect Client A with reason code `0x94` (Topic Alias Invalid) upon receiving the 11th `PUBLISH` packet.

#### ✔️ **Case ID: ALIAS-8.3**
*   **Description**: Verify that the Broker correctly handles the re-mapping of an alias.
*   **MQTT v5.0 Spec Reference**: Section `3.3.2.3.2 Topic Alias` - If the topic name is not empty, the Broker updates the mapping.
*   **Setup**: Client C subscribes to `topic/1` and `topic/2`.
*   **Action**: Client A publishes to `topic/1` with alias 1 -> Client A publishes to `topic/2` with alias 1 -> Client A publishes with an empty topic name and alias 1.
*   **Expected Result**: `topic/1` receives the first message, and `topic/2` receives the next two messages.

#### ✔️ **Case ID: ALIAS-8.4**
*   **Description**: Verify that the Broker closes the connection if a Client sends a PUBLISH packet with Topic Alias = 0.
*   **MQTT v5.0 Spec Reference**: Section `3.3.2.3.4 Topic Alias` - "A Topic Alias of 0 is not permitted. If a Client sends a PUBLISH packet containing a Topic Alias which has the value 0, the Server MUST close the Network Connection [MQTT-3.3.2.3.4-1]."
*   **Setup**: Client A connects to the Broker.
*   **Action**: Client A sends a `PUBLISH` packet with `Topic Alias = 0`.
*   **Expected Result**: The Broker MUST close the Network Connection, and send a DISCONNECT packet with reason code `0x94` (Topic Alias Invalid).

---

## Part 9: Flow Control

**Objective**: To verify that the Broker adheres to the `Receive Maximum` limit.

#### ✔️ **Case ID: FLOW-9.1**
*   **Description**: Verify that the Broker does not exceed the client's `Receive Maximum` limit when sending offline messages.
*   **MQTT v5.0 Spec Reference**: Section `3.2.2.3.3 Receive Maximum` - The sender must not send more unacknowledged QoS>0 messages than this value at any time.
*   **Setup**:
    *   1. Client A connects with `clean_start=false` and `Session Expiry Interval > 0`, and subscribes to `T1` (QoS 1).
    *   2. Client A disconnects.
    *   3. 10 QoS 1 messages are published to topic `T1`.
*   **Action**: Client A reconnects before the session expires, setting `Receive Maximum` to 5 in the `CONNECT` packet.
*   **Expected Result**: The Broker sends a maximum of 5 `PUBLISH` messages, and continues sending the remaining messages only after receiving `PUBACK` from the client.

#### ✔️ **Case ID: FLOW-9.2**
*   **Description**: Verify that the Broker enforces its own `Receive Maximum` limit set in the `CONNACK` packet.
*   **MQTT v5.0 Spec Reference**: Sections `3.2.2.3.3 Receive Maximum` & `4.13.1.2.19 Receive Maximum exceeded`.
*   **Setup**: The Broker declares `Receive Maximum` as 5 to Client A in the `CONNACK` packet.
*   **Action**: Client A continuously publishes 10 QoS 1 messages without waiting for `PUBACK`.
*   **Expected Result**: Upon receiving the 6th `PUBLISH` message, the Broker should disconnect the client with reason code `0x9B` (Receive Maximum exceeded).

#### ✔️ **Case ID: FLOW-9.3**
*   **Description**: Verify that the Broker adheres to the `Maximum Packet Size` limit declared by the client in the `CONNECT` packet.
*   **MQTT v5.0 Spec Reference**: Section `3.1.2.11.3 Maximum Packet Size` - The sender must not send a packet larger than the `Maximum Packet Size` declared by the receiver.
*   **Setup**:
    *   1. Client A connects, setting `Maximum Packet Size` to 100 bytes in the `CONNECT` packet.
    *   2. Client A subscribes to topic `T_PACKET_SIZE`.
*   **Action**:
    *   Client B publishes a message to topic `T_PACKET_SIZE` where the total `PUBLISH` packet size exceeds 100 bytes (e.g., 200 bytes).
*   **Expected Result**:
    *   The Broker **must not** forward the oversized `PUBLISH` packet to Client A.

---

## Part 10: Request/Response Properties

**Objective**: To verify the Broker's support for the request/response pattern using `Response Topic` and `Correlation Data`.

#### ✔️ **Case ID: REQ-10.1**
*   **Description**: Verify that the Broker correctly forwards `Response Topic` and `Correlation Data` to a responding client.
*   **MQTT v5.0 Spec Reference**: Sections `3.3.2.3.6 Response Topic` & `3.3.2.3.7 Correlation Data`.
*   **Setup**: Client B (responder) subscribes to `request/topic`. Client A (requester) subscribes to `response/topic`.
*   **Action**: Client A publishes a message to `request/topic` with `Response Topic` set to `response/topic` and `Correlation Data` set to a specific value (e.g., "request123").
*   **Expected Result**: 1. Client B receives the `PUBLISH` message containing both the `Response Topic` and `Correlation Data` properties. 2. Client B can then publish a response to the received `Response Topic` with the same `Correlation Data`, which Client A will receive.

---

## Part 11: Message Context Properties

**Objective**: To verify that the Broker correctly forwards descriptive message properties.

#### ✔️ **Case ID: CTX-11.1**
*   **Description**: Verify the end-to-end transport of `User Property` in a `PUBLISH` packet.
*   **MQTT v5.0 Spec Reference**: Section `3.3.2.3.9 User Property`.
*   **Setup**: Client B subscribes to `T_USER_PROP`.
*   **Action**: Client A publishes a message to `T_USER_PROP` containing one or more `User Property` key-value pairs.
*   **Expected Result**: Client B receives the `PUBLISH` message with the `User Property` key-value pairs unmodified.

#### ✔️ **Case ID: CTX-11.2**
*   **Description**: Verify the end-to-end transport of `Content Type` and `Payload Format Indicator`.
*   **MQTT v5.0 Spec Reference**: Sections `3.3.2.3.8 Content Type` & `3.3.2.3.5 Payload Format Indicator`.
*   **Setup**: Client B subscribes to `T_CONTENT_TYPE`.
*   **Action**: Client A publishes a message to `T_CONTENT_TYPE` with `Payload Format Indicator = 1` (UTF-8) and `Content Type = "application/json"`.
*   **Expected Result**: Client B receives the `PUBLISH` message with both `Payload Format Indicator` and `Content Type` properties intact.

---

## Part 12: Subscription Properties

**Objective**: To verify the Broker's handling of properties related to subscriptions.

#### ✔️ **Case ID: SUB-12.1**
*   **Description**: Verify the Broker correctly associates and forwards a `Subscription Identifier`.
*   **MQTT v5.0 Spec Reference**: Section `3.8.2.1.2 Subscription Identifier`.
*   **Setup**: Client A subscribes to topic `data/#` with `Subscription Identifier = 10`.
*   **Action**: Client B publishes a message to `data/sensor1`.
*   **Expected Result**: The `PUBLISH` packet received by Client A must contain a `Subscription Identifier` property with the value `10`.

---

## Part 13: Subscription Options

**Objective**: To verify the Broker's handling of specific MQTTv5 subscription options like `Retain Handling` and `Retain as Published`.

#### ✔️ **Case ID: SUB-OPT-13.1 (`Retain Handling = 0`)**
*   **Description**: Verify that when `Retain Handling = 0`, the client receives retained messages on every subscription, even for existing ones.
*   **MQTT v5.0 Spec Reference**: Section `3.8.3.1.4 Retain Handling`.
*   **Setup**: 1. A retained message exists for topic `T_RH`. 2. Client A connects, subscribes to `T_RH`, and receives the retained message. Client A remains connected.
*   **Action**: Client A sends a second `SUBSCRIBE` packet for the same topic `T_RH` with `Retain Handling = 0`.
*   **Expected Result**: Client A **will** receive the retained message again.

#### ✔️ **Case ID: SUB-OPT-13.2 (`Retain Handling = 1`)**
*   **Description**: Verify that when `Retain Handling = 1`, the client only receives retained messages for new subscriptions.
*   **MQTT v5.0 Spec Reference**: Section `3.8.3.1.4 Retain Handling`.
*   **Setup**: (Same as above).
*   **Action**: Client A sends a second `SUBSCRIBE` packet for the same topic `T_RH` with `Retain Handling = 1`.
*   **Expected Result**: Client A **will not** receive the retained message again, as the subscription already exists.

#### ✔️ **Case ID: SUB-OPT-13.3 (`Retain Handling = 2`)**
*   **Description**: Verify that when `Retain Handling = 2`, the client does not receive any retained messages upon subscribing.
*   **MQTT v5.0 Spec Reference**: Section `3.8.3.1.4 Retain Handling`.
*   **Setup**: A retained message exists for topic `T_RH`.
*   **Action**: A new Client B connects and subscribes to `T_RH` with `Retain Handling = 2`.
*   **Expected Result**: Client B **will not** receive the retained message.

#### ✔️ **Case ID: SUB-OPT-13.4 (`Retain as Published = false`)**
*   **Description**: Verify that when `Retain as Published = false` (default), the `RETAIN` flag on the received message is 0.
*   **MQTT v5.0 Spec Reference**: Section `3.8.3.1.5 Retain As Published`.
*   **Setup**: A retained message exists for topic `T_RAP`.
*   **Action**: Client A subscribes to `T_RAP` with the `Retain as Published` option set to `false`.
*   **Expected Result**: The `PUBLISH` packet received by Client A must have its `RETAIN` flag set to `0`.

#### ✔️ **Case ID: SUB-OPT-13.5 (`Retain as Published = true`)**
*   **Description**: Verify that when `Retain as Published = true`, the `RETAIN` flag on the received message is 1.
*   **MQTT v5.0 Spec Reference**: Section `3.8.3.1.5 Retain As Published`.
*   **Setup**: A retained message exists for topic `T_RAP`.
*   **Action**: Client A subscribes to `T_RAP` with the `Retain as Published` option set to `true`.
*   **Expected Result**: The `PUBLISH` packet received by Client A must have its `RETAIN` flag set to `1`.
