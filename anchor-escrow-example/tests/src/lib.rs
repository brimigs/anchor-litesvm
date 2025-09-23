// Test modules demonstrating different approaches to testing with LiteSVM

// Using the new anchor-litesvm crate with native instruction building (recommended)
#[cfg(test)]
mod anchor_litesvm_test;

// Using anchor-client with mock RPC and LiteSVM
#[cfg(test)]
mod anchor_client_with_litesvm_test;

// Using plain LiteSVM without any Anchor helpers
#[cfg(test)]
mod regular_litesvm_test;