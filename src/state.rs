// Copyright (c) 2021 Streamflow Labs Limited <legal@streamflowlabs.com>
//
// This file is part of streamflow-finance/timelock-crate
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License version 3
// as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};

/// The struct containing instructions for initializing a stream
#[repr(C)]
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct StreamInstruction {
    /// Timestamp when the tokens start vesting
    pub start_time: u64,
    /// Timestamp when all tokens are fully vested
    pub end_time: u64,
    /// Initially deposited amount of tokens (currently not used, set to `total_amount` and left for extension in future)
    pub deposited_amount: u64,
    /// Total amount of the tokens in the escrow account if contract is fully vested
    pub total_amount: u64,
    /// Time step (period) in seconds per which the vesting occurs
    pub period: u64,
    /// Vesting contract "cliff" timestamp
    pub cliff: u64,
    /// Amount unlocked at the "cliff" timestamp
    pub cliff_amount: u64,
    /// Whether or not a stream can be canceled by a sender (currently not used, set to TRUE)
    pub is_cancelable_by_sender: bool,
    /// Whether or not a stream can be canceled by a recipient (currently not used, set to FALSE)
    pub is_cancelable_by_recipient: bool,
    /// Whether or not a 3rd party can initiate withdraw in the name of recipient (currently not used, set to FALSE)
    pub is_withdrawal_public: bool,
    /// Whether or not a recipient can transfer the stream (currently not used, set to TRUE)
    pub is_transferable: bool,
    //4 bytes of padding to make the struct size multiple of 64 bits (8 bytes), non-meaningful data.
    pub padding: u32,
}

impl Default for StreamInstruction {
    fn default() -> Self {
        StreamInstruction {
            start_time: 0,
            end_time: 0,
            deposited_amount: 0,
            total_amount: 0,
            period: 1,
            cliff: 0,
            cliff_amount: 0,
            is_cancelable_by_sender: true,
            is_cancelable_by_recipient: false,
            is_withdrawal_public: false,
            is_transferable: true,
            padding: 0,
        }
    }
}

impl StreamInstruction {
    pub fn new(
        start_time: u64,
        end_time: u64,
        total_amount: u64,
        period: u64,
        cliff: u64,
        cliff_amount: u64,
    ) -> Self {
        Self {
            start_time,
            end_time,
            total_amount,
            deposited_amount: total_amount,
            period,
            cliff,
            cliff_amount,
            is_cancelable_by_sender: true,
            is_cancelable_by_recipient: false,
            is_withdrawal_public: false,
            is_transferable: true,
            padding: 0,
        }
    }
}

/// TokenStreamData is the struct containing metadata for an SPL token stream.
#[derive(BorshSerialize, BorshDeserialize, Default, Debug)]
#[repr(C)]
pub struct TokenStreamData {
    /// Magic bytes
    pub magic: u64,
    /// Timestamp when stream was created
    pub created_at: u64,
    /// Amount of funds withdrawn
    pub withdrawn_amount: u64,
    /// Timestamp when stream was canceled (if canceled)
    pub canceled_at: u64,
    /// Timestamp at which stream can be safely canceled by a 3rd party
    /// (Stream is either fully vested or there isn't enough capital to
    /// keep it active)
    pub cancellable_at: u64,
    /// Timestamp of the last withdrawal
    pub last_withdrawn_at: u64,
    /// Pubkey of the stream initializer
    pub sender: Pubkey,
    /// Pubkey of the stream initializer's token account
    pub sender_tokens: Pubkey,
    /// Pubkey of the stream recipient
    pub recipient: Pubkey,
    /// Pubkey of the stream recipient's token account
    pub recipient_tokens: Pubkey,
    /// Pubkey of the token mint
    pub mint: Pubkey,
    /// Pubkey of the account holding the locked tokens
    pub escrow_tokens: Pubkey,
    /// The stream instruction
    pub ix: StreamInstruction,
}

#[allow(clippy::too_many_arguments)]
impl TokenStreamData {
    /// Initialize a new `TokenStreamData` struct.
    pub fn new(
        created_at: u64,
        sender: Pubkey,
        sender_tokens: Pubkey,
        recipient: Pubkey,
        recipient_tokens: Pubkey,
        mint: Pubkey,
        escrow_tokens: Pubkey,
        start_time: u64,
        end_time: u64,
        total_amount: u64,
        period: u64,
        cliff: u64,
        cliff_amount: u64,
    ) -> Self {
        let ix = StreamInstruction {
            start_time,
            end_time,
            deposited_amount: total_amount,
            total_amount,
            period,
            cliff,
            cliff_amount,
            is_cancelable_by_sender: true,
            is_cancelable_by_recipient: false,
            is_withdrawal_public: false,
            is_transferable: true,
            padding: 0,
        };

        Self {
            magic: 0,
            created_at,
            withdrawn_amount: 0,
            canceled_at: 0,
            cancellable_at: end_time,
            last_withdrawn_at: 0,
            sender,
            sender_tokens,
            recipient,
            recipient_tokens,
            mint,
            escrow_tokens,
            ix,
        }
    }

    /// Calculate amount available for withdrawal with given timestamp.
    pub fn available(&self, now: u64) -> u64 {
        if self.ix.start_time > now || self.ix.cliff > now {
            return 0;
        }

        if now >= self.ix.end_time {
            return self.ix.total_amount - self.withdrawn_amount;
        }

        let cliff = if self.ix.cliff > 0 {
            self.ix.cliff
        } else {
            self.ix.start_time
        };

        let cliff_amount = if self.ix.cliff_amount > 0 {
            self.ix.cliff_amount
        } else {
            0
        };

        let num_periods = (self.ix.end_time - cliff) as f64 / self.ix.period as f64;
        let period_amount = (self.ix.total_amount - cliff_amount) as f64 / num_periods;
        let periods_passed = (now - cliff) / self.ix.period;
        (periods_passed as f64 * period_amount) as u64 + cliff_amount - self.withdrawn_amount
    }
}

/// The account-holding struct for the stream initialization instruction
#[derive(Debug)]
pub struct InitializeAccounts<'a> {
    /// The main wallet address of the initializer.
    pub sender: AccountInfo<'a>,
    /// The associated token account address of `sender`.
    pub sender_tokens: AccountInfo<'a>,
    /// The main wallet address of the recipient.
    pub recipient: AccountInfo<'a>,
    /// The associated token account address of `recipient`.
    /// (Can be either empty or initialized).
    pub recipient_tokens: AccountInfo<'a>,
    /// The account holding the stream metadata.
    /// Expects empty (non-initialized) account.
    pub metadata: AccountInfo<'a>,
    /// The escrow account holding the stream funds.
    /// Expects empty (non-initialized) account.
    pub escrow_tokens: AccountInfo<'a>,
    /// The SPL token mint account
    pub mint: AccountInfo<'a>,
    /// The Rent Sysvar account
    pub rent: AccountInfo<'a>,
    /// The SPL program needed in case an associated account
    /// for the new recipient is being created.
    pub token_program: AccountInfo<'a>,
    /// The Associated Token program needed in case associated
    /// account for the new recipient is being created.
    pub associated_token_program: AccountInfo<'a>,
    /// The Solana system program
    pub system_program: AccountInfo<'a>,
}

/// The account-holding struct for the stream withdraw instruction
pub struct WithdrawAccounts<'a> {
    /// Account invoking transaction. Must match `recipient`
    // Same as `recipient` if `is_withdrawal_public == true`, can be any other account otherwise.
    pub withdraw_authority: AccountInfo<'a>,
    /// Sender account is needed to collect the rent for escrow token account after the last withdrawal
    pub sender: AccountInfo<'a>,
    /// Recipient's wallet address
    pub recipient: AccountInfo<'a>,
    /// The associated token account address of a stream `recipient`
    pub recipient_tokens: AccountInfo<'a>,
    /// The account holding the stream metadata
    pub metadata: AccountInfo<'a>,
    /// The escrow account holding the stream funds
    pub escrow_tokens: AccountInfo<'a>,
    /// The SPL token mint account
    pub mint: AccountInfo<'a>,
    /// The SPL token program
    pub token_program: AccountInfo<'a>,
}

/// The account-holding struct for the stream cancel instruction
pub struct CancelAccounts<'a> {
    /// Account invoking cancel. Must match `sender`.
    //Can be either `sender` or `recipient` depending on the value of `is_cancelable_by_sender` and `is_cancelable_by_recipient`, respectively
    pub cancel_authority: AccountInfo<'a>,
    /// The main wallet address of the initializer
    pub sender: AccountInfo<'a>,
    /// The associated token account address of `sender`
    pub sender_tokens: AccountInfo<'a>,
    /// The main wallet address of the recipient
    pub recipient: AccountInfo<'a>,
    /// The associated token account address of `recipient`
    pub recipient_tokens: AccountInfo<'a>,
    /// The account holding the stream metadata
    pub metadata: AccountInfo<'a>,
    /// The escrow account holding the stream funds
    pub escrow_tokens: AccountInfo<'a>,
    /// The SPL token mint account
    pub mint: AccountInfo<'a>,
    /// The SPL token program
    pub token_program: AccountInfo<'a>,
}

/// Accounts needed for updating stream recipient
pub struct TransferAccounts<'a> {
    /// Wallet address of the existing recipient
    pub existing_recipient: AccountInfo<'a>,
    /// New stream beneficiary
    pub new_recipient: AccountInfo<'a>,
    /// New stream beneficiary's token account.
    /// If not initialized, it will be created and
    /// `existing_recipient` is the fee payer
    pub new_recipient_tokens: AccountInfo<'a>,
    /// The account holding the stream metadata
    pub metadata: AccountInfo<'a>,
    /// The escrow account holding the stream funds
    pub escrow_tokens: AccountInfo<'a>,
    /// The SPL token mint account
    pub mint: AccountInfo<'a>,
    /// Rent account
    pub rent: AccountInfo<'a>,
    /// The SPL program needed in case associated account
    /// for the new recipients is being created.
    pub token_program: AccountInfo<'a>,
    /// The Associated Token program needed in case associated
    /// account for the new recipients is being created.
    pub associated_token_program: AccountInfo<'a>,
    /// The Solana system program needed in case associated
    /// account for the new recipients is being created.
    pub system_program: AccountInfo<'a>,
}
