use crate::bus_message::bus_serde::{FromCbor, ToCbor};
use crate::bus_message::message_type::MessageType;

pub trait Payload: ToCbor + FromCbor {
    const MESSAGE_TYPE: MessageType;

    /// If the payload has a specific unique id that can be derived then that logic is implemented here.
    /// For example `NEARBlock` and `AuroraBlock` have a requirement on the unique id:
    /// <https://github.com/aurora-is-near/borealis-spec/blob/master/spec/nats-wrapper.md#meaning-of-borealis-unique-id>
    /// If there is no unique id that can be derived then simply return None.
    fn unique_id(&self) -> Option<[u8; 16]>;
}
