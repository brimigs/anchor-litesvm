use anchor_lang::AnchorSerialize;
use sha2::{Digest, Sha256};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

/// Builds an Anchor instruction with automatic discriminator calculation
///
/// This function handles the Anchor-specific instruction format:
/// - Calculates the 8-byte discriminator from the instruction name
/// - Serializes the instruction arguments using Borsh
/// - Combines them into the complete instruction data
pub fn build_anchor_instruction<T>(
    program_id: &Pubkey,
    instruction_name: &str,
    accounts: Vec<AccountMeta>,
    args: T,
) -> Result<Instruction, Box<dyn std::error::Error>>
where
    T: AnchorSerialize,
{
    // Calculate discriminator using Anchor's method: sha256("global:<instruction_name>")[..8]
    let discriminator = calculate_anchor_discriminator(instruction_name);

    // Serialize the instruction arguments
    let mut data = discriminator.to_vec();
    args.serialize(&mut data)?;

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Calculate the Anchor instruction discriminator
///
/// Anchor uses the first 8 bytes of sha256("global:<instruction_name>")
/// as the instruction discriminator
pub fn calculate_anchor_discriminator(instruction_name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{}", instruction_name));
    let hash = hasher.finalize();

    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash[..8]);
    discriminator
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshSerialize;

    #[test]
    fn test_discriminator_calculation() {
        // Test known discriminator for "make" instruction
        let make_discriminator = calculate_anchor_discriminator("make");
        // First 8 bytes of SHA256("global:make")
        let expected_make = [0x8a, 0xe3, 0xe8, 0x4d, 0xdf, 0xa6, 0x60, 0xc5];
        assert_eq!(make_discriminator, expected_make);

        // Test another instruction to ensure consistency
        let test_discriminator = calculate_anchor_discriminator("test");
        assert_eq!(test_discriminator.len(), 8);
        // Ensure different instructions produce different discriminators
        assert_ne!(make_discriminator, test_discriminator);
    }

    #[test]
    fn test_instruction_building() {
        #[derive(BorshSerialize)]
        struct TestArgs {
            value: u64,
        }

        impl AnchorSerialize for TestArgs {
            fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                BorshSerialize::serialize(self, writer)
            }
        }

        let program_id = Pubkey::new_unique();
        let accounts = vec![
            AccountMeta::new(Pubkey::new_unique(), true),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
        ];
        let args = TestArgs { value: 42 };

        let instruction = build_anchor_instruction(
            &program_id,
            "test",
            accounts.clone(),
            args,
        ).unwrap();

        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 2);
        assert!(instruction.data.len() >= 8); // At least discriminator
    }
}