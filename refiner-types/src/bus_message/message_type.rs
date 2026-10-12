use crate::bus_message::error;

/// Exhaustive list of all known message types. See the spec for details:
/// <https://github.com/aurora-is-near/borealis-spec>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MessageType {
    // Requests/Responses
    RpcRequest = 0x1020,
    RpcResponse = 0x1021,
    ErrorResponse = 0x1100,
    GetAccountStatusRequest = 0x1101,
    GetAccountStatusResponse = 0x1102,
    LookupAccountRequest = 0x1103,
    LookupAccountResponse = 0x1104,

    // Commands
    CreateAccountCommand = 0x0101,
    ChangeAccountEmailCommand = 0x0102,
    ChangeAccountFeaturesCommand = 0x0103,
    ReissueAPITokenCommand = 0x0104,

    // Events
    NEARBlockEventDeprecated = 0x1000,
    NEARBlockEvent = 0x2000,
    AuroraBlockEvent = 0x2010,
    TransactionCounterEvent = 0x2020,
    TransactionQuotaExceededEvent = 0x2021,
    AccountCreatedEvent = 0x2101,
    AccountEmailChangedEvent = 0x2102,
    AccountFeaturesChangedEvent = 0x2103,
    APITokenGrantedEvent = 0x2104,
    APITokenRevokedEvent = 0x2105,
    EOAStakedEvent = 0x2106,
    EOAUnstakedEvent = 0x2107,

    // Experiments
    SendEmailCommand = 0xf001,
}

impl MessageType {
    pub const fn to_u16(self) -> u16 {
        self as u16
    }

    pub const fn try_from_u16(number: u16) -> Result<Self, error::UnknownMessageType> {
        match number {
            0x1020 => Ok(Self::RpcRequest),
            0x1021 => Ok(Self::RpcResponse),
            0x1100 => Ok(Self::ErrorResponse),
            0x1101 => Ok(Self::GetAccountStatusRequest),
            0x1102 => Ok(Self::GetAccountStatusResponse),
            0x1103 => Ok(Self::LookupAccountRequest),
            0x1104 => Ok(Self::LookupAccountResponse),
            0x0101 => Ok(Self::CreateAccountCommand),
            0x0102 => Ok(Self::ChangeAccountEmailCommand),
            0x0103 => Ok(Self::ChangeAccountFeaturesCommand),
            0x0104 => Ok(Self::ReissueAPITokenCommand),
            0x1000 => Ok(Self::NEARBlockEventDeprecated),
            0x2000 => Ok(Self::NEARBlockEvent),
            0x2010 => Ok(Self::AuroraBlockEvent),
            0x2020 => Ok(Self::TransactionCounterEvent),
            0x2021 => Ok(Self::TransactionQuotaExceededEvent),
            0x2101 => Ok(Self::AccountCreatedEvent),
            0x2102 => Ok(Self::AccountEmailChangedEvent),
            0x2103 => Ok(Self::AccountFeaturesChangedEvent),
            0x2104 => Ok(Self::APITokenGrantedEvent),
            0x2105 => Ok(Self::APITokenRevokedEvent),
            0x2106 => Ok(Self::EOAStakedEvent),
            0x2107 => Ok(Self::EOAUnstakedEvent),
            0xf001 => Ok(Self::SendEmailCommand),
            _ => Err(error::UnknownMessageType(number)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MessageType;
    use crate::bus_message::error::UnknownMessageType;

    const MESSAGE_TYPES: &[(MessageType, u16)] = &[
        (MessageType::RpcRequest, 0x1020),
        (MessageType::RpcResponse, 0x1021),
        (MessageType::ErrorResponse, 0x1100),
        (MessageType::GetAccountStatusRequest, 0x1101),
        (MessageType::GetAccountStatusResponse, 0x1102),
        (MessageType::LookupAccountRequest, 0x1103),
        (MessageType::LookupAccountResponse, 0x1104),
        (MessageType::CreateAccountCommand, 0x0101),
        (MessageType::ChangeAccountEmailCommand, 0x0102),
        (MessageType::ChangeAccountFeaturesCommand, 0x0103),
        (MessageType::ReissueAPITokenCommand, 0x0104),
        (MessageType::NEARBlockEventDeprecated, 0x1000),
        (MessageType::NEARBlockEvent, 0x2000),
        (MessageType::AuroraBlockEvent, 0x2010),
        (MessageType::TransactionCounterEvent, 0x2020),
        (MessageType::TransactionQuotaExceededEvent, 0x2021),
        (MessageType::AccountCreatedEvent, 0x2101),
        (MessageType::AccountEmailChangedEvent, 0x2102),
        (MessageType::AccountFeaturesChangedEvent, 0x2103),
        (MessageType::APITokenGrantedEvent, 0x2104),
        (MessageType::APITokenRevokedEvent, 0x2105),
        (MessageType::EOAStakedEvent, 0x2106),
        (MessageType::EOAUnstakedEvent, 0x2107),
        (MessageType::SendEmailCommand, 0xf001),
    ];

    #[test]
    fn test_message_types() {
        for (mt, number) in MESSAGE_TYPES {
            let mt = *mt;
            let number = *number;
            test_to_u16(mt, number);
            test_from_u16(number, Ok(mt));
        }

        let unknown_type: u16 = 0xffff;
        test_from_u16(unknown_type, Err(UnknownMessageType(unknown_type)));
    }

    #[track_caller]
    fn test_to_u16(mt: MessageType, expected_number: u16) {
        assert_eq!(
            mt.to_u16(),
            expected_number,
            "Failed to convert {mt:?} to u16"
        );
    }

    #[track_caller]
    fn test_from_u16(number: u16, expected_mt: Result<MessageType, UnknownMessageType>) {
        let error_message = |got| {
            format!(
                "Failed to parse {number} as MessageType. Expected: {expected_mt:?} Got: {got:?}"
            )
        };
        match MessageType::try_from_u16(number) {
            Ok(mt) => {
                let em = error_message(Ok(mt));
                assert_eq!(mt, expected_mt.expect(&em), "{em}");
            }
            Err(e) => {
                let em = error_message(Err(&e));
                assert_eq!(e.0, expected_mt.expect_err(&em).0, "{em}");
            }
        }
    }
}
