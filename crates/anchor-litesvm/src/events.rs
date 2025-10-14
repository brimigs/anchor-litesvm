//! Event parsing and assertion utilities for Anchor programs
//!
//! This module provides helpers for working with Anchor events in tests.
//! Anchor programs can emit events using the `emit!` macro, and these events
//! are logged during transaction execution.

use anchor_lang::{AnchorDeserialize, Discriminator, Event};
use base64::{engine::general_purpose, Engine as _};
use litesvm_utils::TransactionResult;

/// Event parsing error types
#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("Failed to parse event data: {0}")]
    ParseError(String),

    #[error("Event not found in logs")]
    EventNotFound,

    #[error("Invalid event format")]
    InvalidFormat,

    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("Anchor deserialization error: {0}")]
    AnchorError(String),
}

/// Extension trait for TransactionResult to add event parsing capabilities
pub trait EventHelpers {
    /// Parse all events of a specific type from transaction logs
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[event]
    /// pub struct TransferEvent {
    ///     pub from: Pubkey,
    ///     pub to: Pubkey,
    ///     pub amount: u64,
    /// }
    ///
    /// let result = ctx.execute_instruction(ix, &[&user]).unwrap();
    /// let events: Vec<TransferEvent> = result.parse_events().unwrap();
    /// assert_eq!(events.len(), 1);
    /// assert_eq!(events[0].amount, 1_000_000);
    /// ```
    fn parse_events<T>(&self) -> Result<Vec<T>, EventError>
    where
        T: AnchorDeserialize + Discriminator + Event;

    /// Parse the first event of a specific type from transaction logs
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = ctx.execute_instruction(ix, &[&user]).unwrap();
    /// let event: TransferEvent = result.parse_event().unwrap();
    /// assert_eq!(event.amount, 1_000_000);
    /// ```
    fn parse_event<T>(&self) -> Result<T, EventError>
    where
        T: AnchorDeserialize + Discriminator + Event;

    /// Assert that at least one event of the specified type was emitted
    ///
    /// # Example
    ///
    /// ```ignore
    /// result.assert_event_emitted::<TransferEvent>();
    /// ```
    fn assert_event_emitted<T>(&self)
    where
        T: AnchorDeserialize + Discriminator + Event;

    /// Assert that a specific number of events were emitted
    ///
    /// # Example
    ///
    /// ```ignore
    /// result.assert_event_count::<TransferEvent>(2);
    /// ```
    fn assert_event_count<T>(&self, expected_count: usize)
    where
        T: AnchorDeserialize + Discriminator + Event;

    /// Check if an event of the specified type was emitted
    ///
    /// # Example
    ///
    /// ```ignore
    /// if result.has_event::<TransferEvent>() {
    ///     println!("Transfer event was emitted");
    /// }
    /// ```
    fn has_event<T>(&self) -> bool
    where
        T: AnchorDeserialize + Discriminator + Event;
}

impl EventHelpers for TransactionResult {
    fn parse_events<T>(&self) -> Result<Vec<T>, EventError>
    where
        T: AnchorDeserialize + Discriminator + Event,
    {
        let mut events = Vec::new();

        // Anchor events are logged with the format: "Program data: <base64_encoded_data>"
        // The discriminator for events is the first 8 bytes
        for log in self.logs() {
            if let Some(event_data) = log.strip_prefix("Program data: ") {
                // Decode base64
                let decoded = general_purpose::STANDARD
                    .decode(event_data)
                    .map_err(EventError::Base64Error)?;

                // Check if this matches the event discriminator
                if decoded.len() < 8 {
                    continue;
                }

                let discriminator = &decoded[0..8];
                if discriminator == T::DISCRIMINATOR {
                    // Deserialize the event (skip discriminator)
                    let mut event_data_slice = &decoded[8..];
                    match T::deserialize(&mut event_data_slice) {
                        Ok(event) => events.push(event),
                        Err(e) => {
                            return Err(EventError::AnchorError(e.to_string()));
                        }
                    }
                }
            }
        }

        Ok(events)
    }

    fn parse_event<T>(&self) -> Result<T, EventError>
    where
        T: AnchorDeserialize + Discriminator + Event,
    {
        self.parse_events()?
            .into_iter()
            .next()
            .ok_or(EventError::EventNotFound)
    }

    fn assert_event_emitted<T>(&self)
    where
        T: AnchorDeserialize + Discriminator + Event,
    {
        match self.parse_events::<T>() {
            Ok(events) => {
                assert!(
                    !events.is_empty(),
                    "Expected at least one event of type '{}' to be emitted, but none were found.\nLogs:\n{}",
                    std::any::type_name::<T>(),
                    self.logs().join("\n")
                );
            }
            Err(e) => {
                panic!(
                    "Failed to parse events of type '{}': {}\nLogs:\n{}",
                    std::any::type_name::<T>(),
                    e,
                    self.logs().join("\n")
                );
            }
        }
    }

    fn assert_event_count<T>(&self, expected_count: usize)
    where
        T: AnchorDeserialize + Discriminator + Event,
    {
        match self.parse_events::<T>() {
            Ok(events) => {
                assert_eq!(
                    events.len(),
                    expected_count,
                    "Expected {} events of type '{}', but found {}.\nLogs:\n{}",
                    expected_count,
                    std::any::type_name::<T>(),
                    events.len(),
                    self.logs().join("\n")
                );
            }
            Err(e) => {
                panic!(
                    "Failed to parse events of type '{}': {}\nLogs:\n{}",
                    std::any::type_name::<T>(),
                    e,
                    self.logs().join("\n")
                );
            }
        }
    }

    fn has_event<T>(&self) -> bool
    where
        T: AnchorDeserialize + Discriminator + Event,
    {
        self.parse_events::<T>()
            .map(|events| !events.is_empty())
            .unwrap_or(false)
    }
}

/// Helper function to manually parse event data from a base64-encoded string
///
/// This is useful if you need to parse events from log strings directly.
///
/// # Example
///
/// ```ignore
/// let event_data = "base64_encoded_event_data_here";
/// let event: TransferEvent = parse_event_data(event_data).unwrap();
/// ```
pub fn parse_event_data<T>(base64_data: &str) -> Result<T, EventError>
where
    T: AnchorDeserialize + Discriminator + Event,
{
    // Decode base64
    let decoded = general_purpose::STANDARD
        .decode(base64_data)
        .map_err(EventError::Base64Error)?;

    // Check discriminator
    if decoded.len() < 8 {
        return Err(EventError::InvalidFormat);
    }

    let discriminator = &decoded[0..8];
    if discriminator != T::DISCRIMINATOR {
        return Err(EventError::InvalidFormat);
    }

    // Deserialize
    let mut event_data_slice = &decoded[8..];
    T::deserialize(&mut event_data_slice).map_err(|e| EventError::AnchorError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_error_display() {
        let err = EventError::EventNotFound;
        assert_eq!(err.to_string(), "Event not found in logs");

        let err = EventError::ParseError("test error".to_string());
        assert_eq!(err.to_string(), "Failed to parse event data: test error");
    }
}
