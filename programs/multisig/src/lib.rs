//! An example of a social_recovery to execute arbitrary Solana transactions.
//!
//! This program can be used to allow a social_recovery to govern anything a regular
//! Pubkey can govern. One can use the social_recovery as a BPF program upgrade
//! authority, a mint authority, etc.
//!
//! To use, one must first create a `SocialRecovery` account, specifying two important
//! parameters:
//!
//! 1. Allies - the set of addresses that sign transactions for the social_recovery.
//! 2. Threshold - the number of signers required to execute a transaction.
//!
//! Once the `SocialRecovery` account is created, one can create a `Transaction`
//! account, specifying the parameters for a normal solana transaction.
//!
//! To sign, allies should invoke the `approve` instruction, and finally,
//! the `execute_transaction`, once enough (i.e. `threhsold`) of the allies have
//! signed.

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_lang::solana_program::instruction::Instruction;
use std::convert::Into;

#[program]
pub mod serum_social_recovery_impl {
    use super::*;

    // Initializes a new social_recovery account with a set of allies and a threshold.
    pub fn create_social_recovery(
        ctx: Context<CreateSocialRecovery>,
        allies: Vec<Pubkey>,
        threshold: u64,
        nonce: u8,
    ) -> Result<()> {
        let social_recovery = &mut ctx.accounts.social_recovery;
        social_recovery.allies = allies;
        social_recovery.threshold = threshold;
        social_recovery.nonce = nonce;
        social_recovery.alliance_seqno = 0;
        Ok(())
    }

    // Set allies and threshold at once.
    pub fn set_allies_and_change_threshold<'info>(
        ctx: Context<'_, '_, '_, 'info, Auth<'info>>,
        allies: Vec<Pubkey>,
        threshold: u64,
    ) -> Result<()> {
        set_allies(
            Context::new(ctx.program_id, ctx.accounts, ctx.remaining_accounts),
            allies,
        )?;
        change_threshold(ctx, threshold)
    }

    // Sets the allies field on the social_recovery. The only way this can be invoked
    // is via a recursive call from execute_transaction -> set_allies.
    pub fn set_allies(ctx: Context<Auth>, allies: Vec<Pubkey>) -> Result<()> {
        let social_recovery = &mut ctx.accounts.social_recovery;

        if (allies.len() as u64) < social_recovery.threshold {
            social_recovery.threshold = allies.len() as u64;
        }

        social_recovery.allies = allies;
        social_recovery.alliance_seqno += 1;

        Ok(())
    }

    // Changes the execution threshold of the social_recovery. The only way this can be
    // invoked is via a recursive call from execute_transaction ->
    // change_threshold.
    pub fn change_threshold(ctx: Context<Auth>, threshold: u64) -> Result<()> {
        if threshold > ctx.accounts.social_recovery.allies.len() as u64 {
            return Err(ErrorCode::InvalidThreshold.into());
        }
        let social_recovery = &mut ctx.accounts.social_recovery;
        social_recovery.threshold = threshold;
        Ok(())
    }

    // Executes the given transaction
    pub fn execute_transaction(ctx: Context<ExecuteTransaction>, program_id: Pubkey, accs: Vec<TransactionAccount>, data: Vec<u8>) -> Result<()> {

        let insn = Instruction {
            program_id,
            accounts: accs.iter().map(|a| a.into()).collect(),
            data
        };

        insn.accounts = insn
            .accounts
            .iter()
            .map(|acc| {
                let mut acc = acc.clone();
                if &acc.pubkey == ctx.accounts.signer.key {
                    acc.is_signer = true;
                }
                acc
            })
            .collect();
        let seeds = &[
            ctx.accounts.social_recovery.to_account_info().key.as_ref(),
            &[ctx.accounts.social_recovery.nonce],
        ];

        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;
        solana_program::program::invoke_signed(&insn, accounts, signer)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateSocialRecovery<'info> {
    #[account(init)]
    social_recovery: ProgramAccount<'info, SocialRecovery>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Auth<'info> {
    #[account(mut)]
    social_recovery: ProgramAccount<'info, SocialRecovery>,
    #[account(signer, seeds = [
        social_recovery.to_account_info().key.as_ref(),
        &[social_recovery.nonce],
    ])]
    social_recovery_signer: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ExecuteTransaction<'info> {
    #[account(constraint = &social_recovery.signer == signer.key)]
    social_recovery: ProgramAccount<'info, SocialRecovery>,
    #[account(signer, seeds = [
        social_recovery.to_account_info().key.as_ref(),
        &[social_recovery.nonce],
    ])]
    signer: AccountInfo<'info>,
}

#[account]
pub struct SocialRecovery {
    pub signer: Pubkey,
    pub allies: Vec<Pubkey>,
    pub threshold: u64,
    pub nonce: u8,
    pub alliance_seqno: u32,
}

#[account]
pub struct Transaction {
    // The social_recovery account this transaction belongs to.
    pub social_recovery: Pubkey,
    // Target program to execute against.
    pub program_id: Pubkey,
    // Accounts requried for the transaction.
    pub accounts: Vec<TransactionAccount>,
    // Instruction data for the transaction.
    pub data: Vec<u8>,
    // Boolean ensuring one time execution.
    pub did_execute: bool,
    // sequence number.
    pub alliance_seqno: u32,
}

impl From<&Transaction> for Instruction {
    fn from(tx: &Transaction) -> Instruction {
        Instruction {
            program_id: tx.program_id,
            accounts: tx.accounts.iter().map(AccountMeta::from).collect(),
            data: tx.data.clone(),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransactionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl From<&TransactionAccount> for AccountMeta {
    fn from(account: &TransactionAccount) -> AccountMeta {
        match account.is_writable {
            false => AccountMeta::new_readonly(account.pubkey, account.is_signer),
            true => AccountMeta::new(account.pubkey, account.is_signer),
        }
    }
}

impl From<&AccountMeta> for TransactionAccount {
    fn from(account_meta: &AccountMeta) -> TransactionAccount {
        TransactionAccount {
            pubkey: account_meta.pubkey,
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        }
    }
}

#[error]
pub enum ErrorCode {
    #[msg("The given owner is not part of this social_recovery.")]
    InvalidOwner,
    #[msg("Not enough allies signed this transaction.")]
    NotEnoughSigners,
    #[msg("Cannot delete a transaction that has been signed by an owner.")]
    TransactionAlreadySigned,
    #[msg("Overflow when adding.")]
    Overflow,
    #[msg("Cannot delete a transaction the owner did not create.")]
    UnableToDelete,
    #[msg("The given transaction has already been executed.")]
    AlreadyExecuted,
    #[msg("Threshold must be less than or equal to the number of allies.")]
    InvalidThreshold,
}
